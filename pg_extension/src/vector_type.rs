//! This file defines a `Vector` type.

use pgrx::callconv::ArgAbi;
use pgrx::callconv::BoxRet;
use pgrx::datum::FromDatum;
use pgrx::datum::IntoDatum;
use pgrx::extension_sql;
use pgrx::pg_extern;
use pgrx::pg_sys;
use pgrx::pgrx_sql_entity_graph::metadata::ArgumentError;
use pgrx::pgrx_sql_entity_graph::metadata::Returns;
use pgrx::pgrx_sql_entity_graph::metadata::ReturnsError;
use pgrx::pgrx_sql_entity_graph::metadata::SqlMapping;
use pgrx::pgrx_sql_entity_graph::metadata::SqlTranslatable;
use pgrx::stringinfo::StringInfo;
use pgrx::wrappers::rust_regtypein;
use std::error::Error;
use std::ffi::CStr;
use std::ffi::CString;

/// The `vector` type
#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[serde(transparent)]
struct Vector {
    value: Vec<f64>,
}

unsafe impl SqlTranslatable for Vector {
    fn argument_sql() -> Result<SqlMapping, ArgumentError> {
        Ok(SqlMapping::As("vector".into()))
    }

    fn return_sql() -> Result<Returns, ReturnsError> {
        Ok(Returns::One(SqlMapping::As("vector".into())))
    }
}

impl FromDatum for Vector {
    unsafe fn from_polymorphic_datum(
        datum: pg_sys::Datum,
        is_null: bool,
        typoid: pg_sys::Oid,
    ) -> Option<Self>
    where
        Self: Sized,
    {
        if is_null {
            None
        } else {
            let serialized_str =
                <String as FromDatum>::from_polymorphic_datum(datum, is_null, typoid)
                    .expect("should be Some as is not null");
            let vector = serde_json::from_str(&serialized_str).expect("corrupted serialized str");
            Some(vector)
        }
    }
}

impl IntoDatum for Vector {
    fn into_datum(self) -> Option<pg_sys::Datum> {
        let serialized_str = serde_json::to_string(&self).unwrap();
        <String as IntoDatum>::into_datum(serialized_str)
    }

    fn type_oid() -> pg_sys::Oid {
        rust_regtypein::<Self>()
    }
}

unsafe impl<'fcx> ArgAbi<'fcx> for Vector
where
    Self: 'fcx,
{
    unsafe fn unbox_arg_unchecked(arg: ::pgrx::callconv::Arg<'_, 'fcx>) -> Self {
        unsafe { arg.unbox_arg_using_from_datum().unwrap() }
    }
}

unsafe impl BoxRet for Vector {
    unsafe fn box_into<'fcx>(
        self,
        fcinfo: &mut pgrx::callconv::FcInfo<'fcx>,
    ) -> pgrx::datum::Datum<'fcx> {
        unsafe { fcinfo.return_raw_datum(self.into_datum().expect("should be Some?")) }
    }
}

extension_sql!(
    r#"
CREATE TYPE vector; -- shell type
"#,
    name = "shell_type",
    bootstrap // declare this extension_sql block as the "bootstrap" block so that it happens first in sql generation
);

#[pg_extern(immutable, parallel_safe, requires = [ "shell_type" ])]
fn vector_input(
    input: &CStr,
    _oid: pg_sys::Oid,
    type_modifier: i32,
) -> Result<Vector, Box<dyn Error>> {
    let value = serde_json::from_str::<Vec<f64>>(input.to_str()?)?;
    let dimension = value.len();

    // check the dimension in INPUT function if we know the expected dimension.
    if type_modifier != -1 {
        let expected_dimension = type_modifier as usize;
        if dimension != expected_dimension {
            pgrx::error!(
                "pg_ivfflat: mismatched dimension, expected {}, found {}",
                expected_dimension,
                dimension
            );
        }
    }

    Ok(Vector { value })
}

#[pg_extern(immutable, parallel_safe, requires = [ "shell_type" ])]
fn vector_output(value: Vector) -> &'static CStr {
    let mut s = StringInfo::new();
    let value_serialized_string = serde_json::to_string(&value).unwrap();
    s.push_str(&value_serialized_string);
    // SAFETY: We just constructed this StringInfo ourselves
    unsafe { s.leak_cstr() }
}

#[pgrx::pg_extern(immutable, parallel_safe)]
fn vector_modifier_input(list: pgrx::datum::Array<&CStr>) -> i32 {
    if list.len() != 1 {
        pgrx::error!("pg_ivfflat: too many modifiers, expect 1")
    }

    let modifier = list.get(0).unwrap().unwrap();
    let Ok(dimension) = modifier.to_str().unwrap().parse::<u16>() else {
        pgrx::error!("pg_ivfflat: too many dimensions, expect [1, 65535]")
    };

    dimension as i32
}

#[pgrx::pg_extern(immutable, strict, parallel_safe)]
fn vector_modifier_output(type_modifer: i32) -> CString {
    CString::new(format!("({})", type_modifer)).unwrap()
}

/// Cast a `vector` to a `vector`, the conversion is meaningless, but we do need
/// to do the dimension check here if we cannot get the `typmod` value in vector
/// type's input function.
#[pgrx::pg_extern(immutable, strict, parallel_safe)]
fn cast_vector_to_vector(vector: Vector, type_modifier: i32, _explicit: bool) -> Vector {
    let expected_dimension = u16::try_from(type_modifier).expect("invalid type_modifier") as usize;
    let dimension = vector.value.len();
    if vector.value.len() != expected_dimension {
        pgrx::error!(
            "pg_ivfflat: mismatched dimension, expected {}, found {}",
            type_modifier,
            dimension
        );
    }

    vector
}

extension_sql!(
    r#"
    CREATE CAST (vector AS vector)
    WITH FUNCTION cast_vector_to_vector(vector, integer, boolean);
    "#,
    name = "cast_vector_to_vector",
    requires = ["shell_type", "concrete_type", cast_vector_to_vector]
);

// create the actual type, specifying the input and output functions
extension_sql!(
    r#"
CREATE TYPE vector (
    INPUT = vector_input,
    OUTPUT = vector_output,
    TYPMOD_IN = vector_modifier_input,
    TYPMOD_OUT = vector_modifier_output
);
"#,
    name = "concrete_type",
    creates = [Type(Vector)],
    requires = [
        "shell_type",
        vector_input,
        vector_output,
        vector_modifier_input,
        vector_modifier_output
    ], // so that we won't be created until the shell type and input and output functions have
);
