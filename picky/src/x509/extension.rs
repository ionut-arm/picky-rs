use core::slice::{Iter, IterMut};
use picky_asn1::{
    bit_string::BitString,
    wrapper::{Asn1SequenceOf, BitStringAsn1},
};

use crate::{
    oids,
    x509::private::name::{GeneralName, GeneralNames},
};
use picky_asn1::wrapper::{
    ApplicationTag1, ContextTag0, ContextTag2, Implicit, IntegerAsn1, ObjectIdentifierAsn1, OctetStringAsn1,
    OctetStringAsn1Container,
};
use serde::{de, ser, Deserialize, Serialize};
use std::fmt;

/// https://tools.ietf.org/html/rfc5280#section-4.1.2.9
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Extensions(pub Vec<Extension>);

#[derive(Debug, PartialEq, Clone)]
pub struct Extension {
    extn_id: ObjectIdentifierAsn1,
    critical: Implicit<bool>,
    extn_value: ExtensionValue,
}

impl Extension {
    pub fn extn_id(&self) -> &ObjectIdentifierAsn1 {
        &self.extn_id
    }

    pub fn critical(&self) -> bool {
        self.critical.0
    }

    pub fn extn_value(&self) -> ExtensionView<'_> {
        ExtensionView::from(&self.extn_value)
    }

    pub fn into_critical(mut self) -> Self {
        self.critical = true.into();
        self
    }

    pub fn into_non_critical(mut self) -> Self {
        self.critical = false.into();
        self
    }

    pub fn set_critical(&mut self, critical: bool) {
        self.critical = critical.into();
    }

    /// When present, conforming CAs SHOULD mark this extension as critical
    ///
    /// Default is critical.
    pub(crate) fn new_key_usage(key_usage: KeyUsage) -> Self {
        Self {
            extn_id: oids::key_usage().into(),
            critical: true.into(),
            extn_value: ExtensionValue::KeyUsage(key_usage.into()),
        }
    }

    /// Conforming CAs MUST mark this extension as non-critical
    ///
    /// Default is non-critical.
    pub(crate) fn new_subject_key_identifier<V: Into<Vec<u8>>>(ski: V) -> Self {
        Self {
            extn_id: oids::subject_key_identifier().into(),
            critical: false.into(),
            extn_value: ExtensionValue::SubjectKeyIdentifier(OctetStringAsn1(ski.into()).into()),
        }
    }

    /// Conforming CAs MUST mark this extension as non-critical
    ///
    /// Default is critical.
    pub(crate) fn new_authority_key_identifier<KI, I, SN>(
        key_identifier: KI,
        authority_cert_issuer: I,
        authority_cert_serial_number: SN,
    ) -> Self
    where
        KI: Into<Option<KeyIdentifier>>,
        I: Into<Option<GeneralName>>,
        SN: Into<Option<IntegerAsn1>>,
    {
        Self {
            extn_id: oids::authority_key_identifier().into(),
            critical: false.into(),
            extn_value: ExtensionValue::AuthorityKeyIdentifier(
                AuthorityKeyIdentifier {
                    key_identifier: key_identifier.into().map(ContextTag0),
                    authority_cert_issuer: authority_cert_issuer.into().map(ApplicationTag1),
                    authority_cert_serial_number: authority_cert_serial_number.into().map(ContextTag2),
                }
                .into(),
            ),
        }
    }

    /// Marking this extension as critical is always acceptable.
    /// Check details here: https://tools.ietf.org/html/rfc5280#section-4.2.1.9
    /// You may change this value using `into_non_critical` or `set_critical` methods.
    ///
    /// Default is critical.
    pub(crate) fn new_basic_constraints<CA: Into<Option<bool>>, PLC: Into<Option<u8>>>(
        ca: CA,
        path_len_constraints: PLC,
    ) -> Self {
        Self {
            extn_id: oids::basic_constraints().into(),
            critical: true.into(),
            extn_value: ExtensionValue::BasicConstraints(
                BasicConstraints {
                    ca: Implicit(ca.into()),
                    path_len_constraint: Implicit(path_len_constraints.into()),
                }
                .into(),
            ),
        }
    }

    /// This extension MAY, at the option of the certificate issuer, be either critical or non-critical.
    /// Conforming CAs SHOULD NOT mark this extension as critical if the anyExtendedKeyUsage
    /// KeyPurposeId is present.
    ///
    /// Default is non-critical if anyExtendedKeyUsage is present, critical otherwise.
    pub(crate) fn new_extended_key_usage<EKU>(extended_key_usage: EKU) -> Self
    where
        EKU: Into<ExtendedKeyUsage>,
    {
        let eku = extended_key_usage.into();
        Self {
            extn_id: oids::extended_key_usage().into(),
            critical: Implicit(!eku.contains(oids::kp_any_extended_key_usage())),
            extn_value: ExtensionValue::ExtendedKeyUsage(eku.into()),
        }
    }

    /// If the subject field contains an empty sequence, then the issuing CA MUST include a
    /// subjectAltName extension that is marked as critical. When including
    /// the subjectAltName extension in a certificate that has a non-empty
    /// subject distinguished name, conforming CAs SHOULD mark the
    /// subjectAltName extension as non-critical.
    ///
    /// Default is critical.
    pub(crate) fn new_subject_alt_name<N: Into<SubjectAltName>>(name: N) -> Self {
        let name = name.into();
        Self {
            extn_id: oids::subject_alternative_name().into(),
            critical: true.into(),
            extn_value: ExtensionValue::SubjectAltName(name.into()),
        }
    }

    /// Where present, conforming CAs SHOULD mark this extension as non-critical.
    ///
    /// Default is non-critical.
    pub(crate) fn new_issuer_alt_name<N: Into<IssuerAltName>>(name: N) -> Self {
        let name = name.into();
        Self {
            extn_id: oids::issuer_alternative_name().into(),
            critical: false.into(),
            extn_value: ExtensionValue::IssuerAltName(name.into()),
        }
    }
}

impl ser::Serialize for Extension {
    fn serialize<S>(&self, serializer: S) -> Result<<S as ser::Serializer>::Ok, <S as ser::Serializer>::Error>
    where
        S: ser::Serializer,
    {
        use ser::SerializeSeq;
        let mut seq = serializer.serialize_seq(Some(3))?;
        seq.serialize_element(&self.extn_id)?;

        if self.critical.0 != bool::default() {
            seq.serialize_element(&self.critical)?;
        }

        seq.serialize_element(&self.extn_value)?;

        seq.end()
    }
}

impl<'de> de::Deserialize<'de> for Extension {
    fn deserialize<D>(deserializer: D) -> Result<Self, <D as de::Deserializer<'de>>::Error>
    where
        D: de::Deserializer<'de>,
    {
        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = Extension;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a valid DER-encoded algorithm identifier")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: de::SeqAccess<'de>,
            {
                let id: ObjectIdentifierAsn1 = seq_next_element!(seq, Extension, "id");
                let critical: Implicit<bool> = seq_next_element!(seq, Extension, "critical");
                let value = match Into::<String>::into(&id.0).as_str() {
                    oids::AUTHORITY_KEY_IDENTIFIER => ExtensionValue::AuthorityKeyIdentifier(seq_next_element!(
                        seq,
                        Extension,
                        "AuthorityKeyIdentifier"
                    )),
                    oids::SUBJECT_KEY_IDENTIFIER => {
                        ExtensionValue::SubjectKeyIdentifier(seq_next_element!(seq, Extension, "SubjectKeyIdentifier"))
                    }
                    oids::KEY_USAGE => ExtensionValue::KeyUsage(seq_next_element!(seq, Extension, "KeyUsage")),
                    oids::SUBJECT_ALTERNATIVE_NAME => {
                        ExtensionValue::SubjectAltName(seq_next_element!(seq, Extension, "SubjectAltName"))
                    }
                    oids::ISSUER_ALTERNATIVE_NAME => {
                        ExtensionValue::IssuerAltName(seq_next_element!(seq, Extension, "IssuerAltName"))
                    }
                    oids::BASIC_CONSTRAINTS => {
                        ExtensionValue::BasicConstraints(seq_next_element!(seq, Extension, "BasicConstraints"))
                    }
                    oids::EXTENDED_KEY_USAGE => {
                        ExtensionValue::ExtendedKeyUsage(seq_next_element!(seq, Extension, "ExtendedKeyUsage"))
                    }
                    _ => ExtensionValue::Generic(seq_next_element!(seq, Extension, "Generic")),
                };

                Ok(Extension {
                    extn_id: id,
                    critical,
                    extn_value: value,
                })
            }
        }

        deserializer.deserialize_seq(Visitor)
    }
}

/// A view on an Extension's value designed to be easier to use match and use.
#[derive(Debug, PartialEq, Clone)]
pub enum ExtensionView<'a> {
    AuthorityKeyIdentifier(&'a AuthorityKeyIdentifier),
    SubjectKeyIdentifier(&'a SubjectKeyIdentifier),
    KeyUsage(&'a KeyUsage),
    SubjectAltName(super::name::GeneralNames),
    IssuerAltName(super::name::GeneralNames),
    BasicConstraints(&'a BasicConstraints),
    ExtendedKeyUsage(&'a ExtendedKeyUsage),
    Generic(&'a OctetStringAsn1),
}

impl<'a> From<&'a ExtensionValue> for ExtensionView<'a> {
    fn from(value: &'a ExtensionValue) -> Self {
        match value {
            ExtensionValue::AuthorityKeyIdentifier(OctetStringAsn1Container(val)) => Self::AuthorityKeyIdentifier(val),
            ExtensionValue::SubjectKeyIdentifier(OctetStringAsn1Container(val)) => Self::SubjectKeyIdentifier(val),
            ExtensionValue::KeyUsage(OctetStringAsn1Container(val)) => Self::KeyUsage(val),
            ExtensionValue::SubjectAltName(OctetStringAsn1Container(val)) => Self::SubjectAltName(val.clone().into()),
            ExtensionValue::IssuerAltName(OctetStringAsn1Container(val)) => Self::IssuerAltName(val.clone().into()),
            ExtensionValue::BasicConstraints(OctetStringAsn1Container(val)) => Self::BasicConstraints(val),
            ExtensionValue::ExtendedKeyUsage(OctetStringAsn1Container(val)) => Self::ExtendedKeyUsage(val),
            ExtensionValue::Generic(val) => Self::Generic(val),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
enum ExtensionValue {
    AuthorityKeyIdentifier(OctetStringAsn1Container<AuthorityKeyIdentifier>),
    SubjectKeyIdentifier(OctetStringAsn1Container<SubjectKeyIdentifier>),
    KeyUsage(OctetStringAsn1Container<KeyUsage>),
    //CertificatePolicies(OctetStringAsn1Container<Asn1SequenceOf<PolicyInformation>>),
    //PolicyMappings(OctetStringAsn1Container<Asn1SequenceOfPolicyMapping>>),
    SubjectAltName(OctetStringAsn1Container<SubjectAltName>),
    IssuerAltName(OctetStringAsn1Container<IssuerAltName>),
    //SubjectDirectoryAttributes(OctetStringAsn1Container<Asn1SequenceOf<Attribute>>),
    BasicConstraints(OctetStringAsn1Container<BasicConstraints>),
    //NameConstraints(…),
    //PolicyConstraints(…),
    ExtendedKeyUsage(OctetStringAsn1Container<ExtendedKeyUsage>),
    //CRLDistributionPoints(…),
    //InhibitAnyPolicy(…),
    //FreshestCRL(…),
    Generic(OctetStringAsn1),
}

impl ser::Serialize for ExtensionValue {
    fn serialize<S>(&self, serializer: S) -> Result<<S as ser::Serializer>::Ok, <S as ser::Serializer>::Error>
    where
        S: ser::Serializer,
    {
        match self {
            ExtensionValue::AuthorityKeyIdentifier(aki) => aki.serialize(serializer),
            ExtensionValue::SubjectKeyIdentifier(ski) => ski.serialize(serializer),
            ExtensionValue::KeyUsage(key_usage) => key_usage.serialize(serializer),
            ExtensionValue::SubjectAltName(san) => san.serialize(serializer),
            ExtensionValue::IssuerAltName(ian) => ian.serialize(serializer),
            ExtensionValue::BasicConstraints(basic_constraints) => basic_constraints.serialize(serializer),
            ExtensionValue::ExtendedKeyUsage(eku) => eku.serialize(serializer),
            ExtensionValue::Generic(octet_string) => octet_string.serialize(serializer),
        }
    }
}

/// https://tools.ietf.org/html/rfc5280#section-4.2.1.1
#[derive(Serialize, Debug, PartialEq, Clone)]
pub struct AuthorityKeyIdentifier {
    key_identifier: Option<ContextTag0<KeyIdentifier>>,
    authority_cert_issuer: Option<ApplicationTag1<GeneralName>>,
    authority_cert_serial_number: Option<ContextTag2<IntegerAsn1>>,
}

impl AuthorityKeyIdentifier {
    pub fn key_identifier(&self) -> Option<&[u8]> {
        self.key_identifier.as_ref().map(|ki| (ki.0).0.as_slice())
    }

    pub fn authority_cert_issuer(&self) -> Option<super::name::GeneralName> {
        self.authority_cert_issuer.as_ref().map(|aci| aci.clone().0.into())
    }

    pub fn authority_cert_serial_number(&self) -> Option<&IntegerAsn1> {
        self.authority_cert_serial_number.as_ref().map(|acsn| &acsn.0)
    }
}

pub type KeyIdentifier = OctetStringAsn1;

impl<'de> de::Deserialize<'de> for AuthorityKeyIdentifier {
    fn deserialize<D>(deserializer: D) -> Result<Self, <D as de::Deserializer<'de>>::Error>
    where
        D: de::Deserializer<'de>,
    {
        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = AuthorityKeyIdentifier;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a valid DER-encoded algorithm identifier")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: de::SeqAccess<'de>,
            {
                Ok(AuthorityKeyIdentifier {
                    key_identifier: seq.next_element().unwrap_or(Some(None)).unwrap_or(None),
                    authority_cert_issuer: seq.next_element().unwrap_or(Some(None)).unwrap_or(None),
                    authority_cert_serial_number: seq.next_element().unwrap_or(Some(None)).unwrap_or(None),
                })
            }
        }

        deserializer.deserialize_seq(Visitor)
    }
}

/// https://tools.ietf.org/html/rfc5280#section-4.2.1.2
pub type SubjectKeyIdentifier = OctetStringAsn1;

/// https://tools.ietf.org/html/rfc5280#section-4.2.1.3
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct KeyUsage(BitStringAsn1);

impl Default for KeyUsage {
    fn default() -> Self {
        Self::new(9)
    }
}

macro_rules! bit_string_get_set {
    ($getter:ident , $setter:ident , $idx:literal) => {
        pub fn $getter(&self) -> bool {
            self.0.is_set($idx)
        }

        pub fn $setter(&mut self, val: bool) {
            if self.0.get_num_bits() <= $idx {
                self.0.set_num_bits($idx + 1)
            }
            self.0.set($idx, val);
        }
    };
    ( $( $getter:ident , $setter:ident , $idx:literal ; )+ ) => {
        $( bit_string_get_set! { $getter, $setter, $idx } )+
    };
}

impl KeyUsage {
    pub fn new(num_bits: usize) -> Self {
        Self(BitString::with_len(num_bits).into())
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.0.payload_view()
    }

    bit_string_get_set! {
        digital_signature, set_digital_signature, 0;
        content_commitment, set_content_commitment, 1;
        key_encipherment, set_key_encipherment, 2;
        data_encipherment, set_data_encipherment, 3;
        key_agreement, set_key_agreement, 4;
        key_cert_sign, set_key_cert_sign, 5;
        crl_sign, set_crl_sign, 6;
        encipher_only, set_encipher_only, 7;
        decipher_only, set_decipher_only, 8;
    }
}

/// https://tools.ietf.org/html/rfc5280#section-4.2.1.6
type SubjectAltName = GeneralNames;

/// https://tools.ietf.org/html/rfc5280#section-4.2.1.7
type IssuerAltName = GeneralNames;

/// https://tools.ietf.org/html/rfc5280#section-4.2.1.9
#[derive(Serialize, Debug, PartialEq, Clone)]
pub struct BasicConstraints {
    ca: Implicit<Option<bool>>, // default is false
    path_len_constraint: Implicit<Option<u8>>,
}

impl BasicConstraints {
    pub fn ca(&self) -> Option<bool> {
        self.ca.0
    }

    pub fn pathlen(&self) -> Option<u8> {
        self.path_len_constraint.0
    }
}

impl<'de> de::Deserialize<'de> for BasicConstraints {
    fn deserialize<D>(deserializer: D) -> Result<Self, <D as de::Deserializer<'de>>::Error>
    where
        D: de::Deserializer<'de>,
    {
        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = BasicConstraints;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a valid DER-encoded basic constraints extension")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: de::SeqAccess<'de>,
            {
                Ok(BasicConstraints {
                    ca: Implicit(seq.next_element().unwrap_or(Some(None)).unwrap_or(None)),
                    path_len_constraint: Implicit(seq.next_element().unwrap_or(Some(None)).unwrap_or(None)),
                })
            }
        }

        deserializer.deserialize_seq(Visitor)
    }
}

/// https://tools.ietf.org/html/rfc5280#section-4.2.1.12
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct ExtendedKeyUsage(Asn1SequenceOf<ObjectIdentifierAsn1>);

impl<OID: Into<ObjectIdentifierAsn1>> From<Vec<OID>> for ExtendedKeyUsage {
    fn from(purpose_oids: Vec<OID>) -> Self {
        ExtendedKeyUsage::new(purpose_oids)
    }
}

impl ExtendedKeyUsage {
    pub fn new<OID: Into<ObjectIdentifierAsn1>>(purpose_oids: Vec<OID>) -> Self {
        Self(
            purpose_oids
                .into_iter()
                .map(|oid| oid.into())
                .collect::<Vec<_>>()
                .into(),
        )
    }

    pub fn iter(&self) -> Iter<ObjectIdentifierAsn1> {
        (self.0).0.iter()
    }

    pub fn iter_mut(&mut self) -> IterMut<ObjectIdentifierAsn1> {
        (self.0).0.iter_mut()
    }

    pub fn contains<C: PartialEq<oid::ObjectIdentifier>>(&self, item: C) -> bool {
        (self.0).0.iter().any(|id| item.eq(&id.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{pem::Pem, x509::private::name::GeneralName};
    use picky_asn1::restricted_string::IA5String;

    #[test]
    fn key_usage() {
        let encoded: [u8; 4] = [0x03, 0x02, 0x01, 0xA0];
        let mut key_usage = KeyUsage::new(7);
        key_usage.set_digital_signature(true);
        key_usage.set_key_encipherment(true);
        assert_eq!(key_usage.as_bytes(), &[0xA0]);
        check_serde!(key_usage: KeyUsage in encoded);
    }

    const CSR_PEM: &str = "-----BEGIN CERTIFICATE REQUEST-----\n\
                           MIIDIjCCAgoCAQAwIDELMAkGA1UEBhMCRlIxETAPBgNVBAMMCERyYXBlYXUhMIIB\n\
                           IjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEA5GqDEM7AfctJizsFEqtAvXd5\n\
                           Fl1GtyXDAnx68MUTuSL22t8aBZoCCi3/9AlS75uUqKggHnRuY2MRYPQaUzpE1F1a\n\
                           aZJNr6tXQy39FtdXrDq2zfwZdDmLW6sPmhvJrBO4yWjuG3wh1paPHy+rBHOjYt+9\n\
                           Pbl/FmDDjIzF8B2LZDuLdnS94Fs/JhogJL/XF4b6RLW60gEnYFjL+ebYdV/f3JYi\n\
                           ccQxY4imvbB2URlIO3t+aG9WMmhHZbbOi/HBdFG1fB7Hsa9Ek2FXshULzEDCJcMz\n\
                           n8HD96XbVBmlaz9nYIcZ83eCOhra67FfFy4pIE1M9saxYJg/OJrMHG12r89yUQID\n\
                           AQABoIG8MIG5BgkqhkiG9w0BCQ4xgaswgagwCQYDVR0TBAIwADALBgNVHQ8EBAMC\n\
                           BeAwJwYDVR0lBCAwHgYIKwYBBQUHAwIGCCsGAQUFBwMBBggrBgEFBQcDAzBlBgNV\n\
                           HREEXjBcghFkZXZlbC5leGFtcGxlLmNvbYIQaXB2Ni5leGFtcGxlLmNvbYIQaXB2\n\
                           NC5leGFtcGxlLmNvbYIQdGVzdC5leGFtcGxlLmNvbYIRcGFydHkuZXhhbXBsZS5j\n\
                           b20wDQYJKoZIhvcNAQELBQADggEBANaSDnpQUGcGypAaafJKAGME2Od8F4pvKjKF\n\
                           lREoWC7JFGIGE/pUrnvrE7qIFmCM3mnFWXEHvResFsdPmEWar+1jMdFinxBg0+J+\n\
                           Op0fxOwfHpxs++8hPsQgnDdL9pIjYFwmIAm64jnyq6wsYIl5CpkvBjGVRVddXkTb\n\
                           VDWhWaGncSdDur6++dp2OAGYTAv4XIHc0nhtcBoxeL4VhjcuksOdGg3JF02gW6Rc\n\
                           B1gipqD0jun8kPgWcQY22zhmP2HuPp0y58t9cu9FsnUcAFa//5pQA1LuaSFp65D4\n\
                           92uaByS3lH18xzrkygzn1BeHRpo0fk4I9Rk8uy2QygCk43Pv6SU=\n\
                           -----END CERTIFICATE REQUEST-----";

    #[test]
    fn eku_ku_bc_san_extensions() {
        let pem = CSR_PEM.parse::<Pem>().expect("couldn't parse PEM");
        let encoded = &pem.data()[359..359 + 3 + 168];

        let mut key_usage = KeyUsage::new(3);
        key_usage.set_digital_signature(true);
        key_usage.set_content_commitment(true);
        key_usage.set_key_encipherment(true);

        let extensions = Extensions(vec![
            Extension::new_basic_constraints(None, None).into_non_critical(),
            Extension::new_key_usage(key_usage).into_non_critical(),
            Extension::new_extended_key_usage(vec![
                oids::kp_client_auth(),
                oids::kp_server_auth(),
                oids::kp_code_signing(),
            ])
            .into_non_critical(),
            Extension::new_subject_alt_name(vec![
                GeneralName::DNSName(IA5String::from_string("devel.example.com".into()).unwrap().into()),
                GeneralName::DNSName(IA5String::from_string("ipv6.example.com".into()).unwrap().into()),
                GeneralName::DNSName(IA5String::from_string("ipv4.example.com".into()).unwrap().into()),
                GeneralName::DNSName(IA5String::from_string("test.example.com".into()).unwrap().into()),
                GeneralName::DNSName(IA5String::from_string("party.example.com".into()).unwrap().into()),
            ])
            .into_non_critical(),
        ]);

        check_serde!(extensions: Extensions in encoded);
    }
}
