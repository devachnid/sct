//! FHIR R4 response builders and the error type, hand-rolled with `serde_json`
//! (no FHIR model crate). See `specs/commands/serve.md`.

use serde_json::{json, Value};

/// SNOMED CT code system URI.
pub const SNOMED_SYSTEM: &str = "http://snomed.info/sct";

/// An error that maps to an HTTP status plus a FHIR `OperationOutcome`.
#[derive(Debug)]
pub struct FhirError {
    pub status: u16,
    pub code: &'static str,
    pub diagnostics: String,
}

impl FhirError {
    pub fn not_found(d: impl Into<String>) -> Self {
        Self {
            status: 404,
            code: "not-found",
            diagnostics: d.into(),
        }
    }
    pub fn invalid(d: impl Into<String>) -> Self {
        Self {
            status: 400,
            code: "invalid",
            diagnostics: d.into(),
        }
    }
    pub fn exception(d: impl Into<String>) -> Self {
        Self {
            status: 500,
            code: "exception",
            diagnostics: d.into(),
        }
    }
    /// The `OperationOutcome` body for this error.
    pub fn outcome(&self) -> Value {
        operation_outcome("error", self.code, &self.diagnostics)
    }
}

/// A FHIR `OperationOutcome` with a single issue.
pub fn operation_outcome(severity: &str, code: &str, diagnostics: &str) -> Value {
    json!({
        "resourceType": "OperationOutcome",
        "issue": [{ "severity": severity, "code": code, "diagnostics": diagnostics }],
    })
}

/// Wrap a list of `parameter` entries in a FHIR `Parameters` resource.
pub fn parameters(parameter: Vec<Value>) -> Value {
    json!({ "resourceType": "Parameters", "parameter": parameter })
}

/// A `$lookup` `property` entry whose value is a coded concept (parent / child /
/// ancestor), with a human-readable description part.
pub fn property_concept(code: &str, sctid: &str, display: &str) -> Value {
    json!({
        "name": "property",
        "part": [
            { "name": "code", "valueCode": code },
            { "name": "value", "valueCode": sctid },
            { "name": "description", "valueString": display },
        ],
    })
}

/// A `$lookup` `designation` entry (FSN or synonym).
pub fn designation(type_id: &str, type_label: &str, term: &str) -> Value {
    json!({
        "name": "designation",
        "part": [
            { "name": "use", "valueCoding": { "system": SNOMED_SYSTEM, "code": type_id, "display": type_label } },
            { "name": "value", "valueString": term },
        ],
    })
}

/// The `/metadata` CapabilityStatement.
pub fn capability_statement(software_version: &str, impl_url: &str) -> Value {
    json!({
        "resourceType": "CapabilityStatement",
        "status": "active",
        "fhirVersion": "4.0.1",
        "kind": "instance",
        "format": ["application/fhir+json", "json"],
        "software": { "name": "sct", "version": software_version },
        "implementation": {
            "description": "SNOMED CT FHIR R4 terminology server backed by SQLite",
            "url": impl_url,
        },
        "rest": [{
            "mode": "server",
            "resource": [
                {
                    "type": "CodeSystem",
                    "operation": [
                        { "name": "lookup", "definition": "http://hl7.org/fhir/OperationDefinition/CodeSystem-lookup" },
                        { "name": "validate-code", "definition": "http://hl7.org/fhir/OperationDefinition/CodeSystem-validate-code" },
                        { "name": "subsumes", "definition": "http://hl7.org/fhir/OperationDefinition/CodeSystem-subsumes" },
                    ],
                },
                {
                    "type": "ValueSet",
                    "operation": [
                        { "name": "expand", "definition": "http://hl7.org/fhir/OperationDefinition/ValueSet-expand" },
                    ],
                },
            ],
        }],
    })
}

/// A FHIR `ValueSet` with an `expansion`. `contains` entries are pre-built.
pub fn value_set_expansion(
    total: usize,
    offset: usize,
    count: usize,
    contains: Vec<Value>,
) -> Value {
    json!({
        "resourceType": "ValueSet",
        "status": "active",
        "expansion": {
            "total": total,
            "offset": offset,
            "parameter": [{ "name": "count", "valueInteger": count }],
            "contains": contains,
        },
    })
}
