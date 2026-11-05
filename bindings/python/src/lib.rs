use appsync_scalars::{Reason, Scalar, validate};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyBool, PyInt, PyString};

fn check_text(scalar: Scalar, value: &Bound<'_, PyString>) -> Result<(), String> {
    // A string with lone surrogates has no UTF-8 form, and no valid value contains them.
    let text = value
        .to_cow()
        .map_err(|_| format!("invalid {}: format not recognized", scalar.name()))?;
    validate(scalar, &text).map_err(|error| error.to_string())
}

fn check_timestamp(value: &Bound<'_, PyAny>) -> Result<(), String> {
    // bool is a subclass of int in Python, but true is not a timestamp.
    if value.is_instance_of::<PyBool>() || !value.is_instance_of::<PyInt>() {
        return Err("invalid AWSTimestamp: expected an integer".to_string());
    }
    // The integer's own value counts, not what a subclass's __str__ or __index__ says, and a
    // value too large for i64 is out of range without asking Python to print every digit.
    let Ok(timestamp) = value.extract::<i64>() else {
        let name = Scalar::Timestamp.name();
        return Err(format!("invalid {name}: {}", Reason::TimestampRange.message()));
    };
    validate(Scalar::Timestamp, &timestamp.to_string()).map_err(|error| error.to_string())
}

macro_rules! text_scalar {
    ($is_valid:ident, $validate:ident, $scalar:expr) => {
        #[pyfunction]
        fn $is_valid(value: &Bound<'_, PyString>) -> bool {
            check_text($scalar, value).is_ok()
        }

        #[pyfunction]
        fn $validate(value: &Bound<'_, PyString>) -> PyResult<()> {
            check_text($scalar, value).map_err(PyValueError::new_err)
        }
    };
}

text_scalar!(is_aws_date, validate_aws_date, Scalar::Date);
text_scalar!(is_aws_time, validate_aws_time, Scalar::Time);
text_scalar!(is_aws_datetime, validate_aws_datetime, Scalar::DateTime);

#[pyfunction]
fn is_aws_timestamp(value: &Bound<'_, PyAny>) -> bool {
    check_timestamp(value).is_ok()
}

#[pyfunction]
fn validate_aws_timestamp(value: &Bound<'_, PyAny>) -> PyResult<()> {
    check_timestamp(value).map_err(PyValueError::new_err)
}

#[pymodule]
fn _native(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(is_aws_date, module)?)?;
    module.add_function(wrap_pyfunction!(is_aws_time, module)?)?;
    module.add_function(wrap_pyfunction!(is_aws_datetime, module)?)?;
    module.add_function(wrap_pyfunction!(is_aws_timestamp, module)?)?;
    module.add_function(wrap_pyfunction!(validate_aws_date, module)?)?;
    module.add_function(wrap_pyfunction!(validate_aws_time, module)?)?;
    module.add_function(wrap_pyfunction!(validate_aws_datetime, module)?)?;
    module.add_function(wrap_pyfunction!(validate_aws_timestamp, module)?)?;
    Ok(())
}
