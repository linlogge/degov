use degov_core::Nsid;
use crate::schema::TypeValidatorDef;

pub fn create_nsid_validator() -> TypeValidatorDef {
    TypeValidatorDef::new("nsid", |value| {
        let string = value.as_string().ok_or(String::from("nsid must be a string"))?;
        Nsid::parse(string).map_err(|e| e.to_string()).map(|_| ())
    })
}
