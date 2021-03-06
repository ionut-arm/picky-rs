use crate::oids;
use oid::ObjectIdentifier;
use picky_asn1::{
    tag::{Tag, TagPeeker},
    wrapper::ObjectIdentifierAsn1,
};
use serde::{de, ser};
use std::fmt;

#[derive(Debug, PartialEq, Clone)]
pub struct AlgorithmIdentifier {
    algorithm: ObjectIdentifierAsn1,
    parameters: AlgorithmIdentifierParameters,
}

impl AlgorithmIdentifier {
    pub fn oid(&self) -> &ObjectIdentifier {
        &self.algorithm.0
    }

    pub fn parameters(&self) -> &AlgorithmIdentifierParameters {
        &self.parameters
    }

    pub fn is_a(&self, algorithm: ObjectIdentifier) -> bool {
        algorithm.eq(&self.algorithm.0)
    }

    pub fn new_sha1_with_rsa_encryption() -> Self {
        Self {
            algorithm: oids::sha1_with_rsa_encryption().into(),
            parameters: AlgorithmIdentifierParameters::Null,
        }
    }

    pub fn new_sha224_with_rsa_encryption() -> Self {
        Self {
            algorithm: oids::sha224_with_rsa_encryption().into(),
            parameters: AlgorithmIdentifierParameters::Null,
        }
    }

    pub fn new_sha256_with_rsa_encryption() -> Self {
        Self {
            algorithm: oids::sha256_with_rsa_encryption().into(),
            parameters: AlgorithmIdentifierParameters::Null,
        }
    }

    pub fn new_sha384_with_rsa_encryption() -> Self {
        Self {
            algorithm: oids::sha384_with_rsa_encryption().into(),
            parameters: AlgorithmIdentifierParameters::Null,
        }
    }

    pub fn new_sha512_with_rsa_encryption() -> Self {
        Self {
            algorithm: oids::sha512_with_rsa_encryption().into(),
            parameters: AlgorithmIdentifierParameters::Null,
        }
    }

    pub fn new_rsa_encryption() -> Self {
        Self {
            algorithm: oids::rsa_encryption().into(),
            parameters: AlgorithmIdentifierParameters::Null,
        }
    }

    pub fn new_ecdsa_with_sha384() -> Self {
        Self {
            algorithm: oids::ecdsa_with_sha384().into(),
            parameters: AlgorithmIdentifierParameters::None,
        }
    }

    pub fn new_ecdsa_with_sha256() -> Self {
        Self {
            algorithm: oids::ecdsa_with_sha256().into(),
            parameters: AlgorithmIdentifierParameters::None,
        }
    }

    pub fn new_elliptic_curve<P: Into<ECParameters>>(ec_params: P) -> Self {
        Self {
            algorithm: oids::ec_public_key().into(),
            parameters: AlgorithmIdentifierParameters::EC(ec_params.into()),
        }
    }
}

impl ser::Serialize for AlgorithmIdentifier {
    fn serialize<S>(&self, serializer: S) -> Result<<S as ser::Serializer>::Ok, <S as ser::Serializer>::Error>
    where
        S: ser::Serializer,
    {
        use ser::SerializeSeq;
        let mut seq = serializer.serialize_seq(Some(2))?;
        seq.serialize_element(&self.algorithm)?;
        match &self.parameters {
            AlgorithmIdentifierParameters::None => {}
            AlgorithmIdentifierParameters::Null => {
                seq.serialize_element(&())?;
            }
            AlgorithmIdentifierParameters::EC(ec_params) => {
                seq.serialize_element(ec_params)?;
            }
        }
        seq.end()
    }
}

impl<'de> de::Deserialize<'de> for AlgorithmIdentifier {
    fn deserialize<D>(deserializer: D) -> Result<Self, <D as de::Deserializer<'de>>::Error>
    where
        D: de::Deserializer<'de>,
    {
        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = AlgorithmIdentifier;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a valid DER-encoded algorithm identifier")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: de::SeqAccess<'de>,
            {
                let oid: ObjectIdentifierAsn1 = seq_next_element!(seq, AlgorithmIdentifier, "algorithm oid");

                let args = match Into::<String>::into(&oid.0).as_str() {
                    oids::RSA_ENCRYPTION
                    | oids::SHA1_WITH_RSA_ENCRYPTION
                    | oids::SHA224_WITH_RSA_ENCRYPTION
                    | oids::SHA256_WITH_RSA_ENCRYPTION
                    | oids::SHA384_WITH_RSA_ENCRYPTION
                    | oids::SHA512_WITH_RSA_ENCRYPTION => {
                        seq_next_element!(seq, AlgorithmIdentifier, "algorithm identifier parameters (null)");
                        AlgorithmIdentifierParameters::Null
                    }
                    oids::ECDSA_WITH_SHA384 | oids::ECDSA_WITH_SHA256 => AlgorithmIdentifierParameters::None,
                    oids::EC_PUBLIC_KEY => AlgorithmIdentifierParameters::EC(seq_next_element!(
                        seq,
                        AlgorithmIdentifier,
                        "elliptic curves parameters"
                    )),
                    _ => {
                        return Err(serde_invalid_value!(
                            AlgorithmIdentifier,
                            "unsupported algorithm (unknown oid)",
                            "a supported algorithm"
                        ));
                    }
                };

                Ok(AlgorithmIdentifier {
                    algorithm: oid,
                    parameters: args,
                })
            }
        }

        deserializer.deserialize_seq(Visitor)
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum AlgorithmIdentifierParameters {
    None,
    Null,
    EC(ECParameters),
}

#[derive(Debug, PartialEq, Clone)]
pub enum ECParameters {
    NamedCurve(ObjectIdentifierAsn1),
    ImplicitCurve,
    //SpecifiedCurve(SpecifiedECDomain) // see [X9.62]
}

impl From<ObjectIdentifierAsn1> for ECParameters {
    fn from(oid: ObjectIdentifierAsn1) -> Self {
        Self::NamedCurve(oid)
    }
}

impl From<ObjectIdentifier> for ECParameters {
    fn from(oid: ObjectIdentifier) -> Self {
        Self::NamedCurve(oid.into())
    }
}

impl From<()> for ECParameters {
    fn from(_: ()) -> Self {
        Self::ImplicitCurve
    }
}

impl ser::Serialize for ECParameters {
    fn serialize<S>(&self, serializer: S) -> Result<<S as ser::Serializer>::Ok, <S as ser::Serializer>::Error>
    where
        S: ser::Serializer,
    {
        match &self {
            ECParameters::NamedCurve(oid) => oid.serialize(serializer),
            ECParameters::ImplicitCurve => ().serialize(serializer),
        }
    }
}

impl<'de> de::Deserialize<'de> for ECParameters {
    fn deserialize<D>(deserializer: D) -> Result<Self, <D as de::Deserializer<'de>>::Error>
    where
        D: de::Deserializer<'de>,
    {
        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = ECParameters;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a valid DER-encoded DirectoryString")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: de::SeqAccess<'de>,
            {
                let tag_peeker: TagPeeker = seq_next_element!(seq, ECParameters, "choice tag");
                match tag_peeker.next_tag {
                    Tag::OID => Ok(ECParameters::NamedCurve(seq_next_element!(
                        seq,
                        ECParameters,
                        "Object Identifier"
                    ))),
                    Tag::NULL => {
                        seq.next_element::<()>()?.expect("should not panic");
                        Ok(ECParameters::ImplicitCurve)
                    }
                    _ => Err(serde_invalid_value!(
                        ECParameters,
                        "unsupported or unknown elliptic curve parameter",
                        "a supported elliptic curve parameter"
                    )),
                }
            }
        }

        deserializer.deserialize_enum("DirectoryString", &["NamedCurve", "ImplicitCurve"], Visitor)
    }
}
