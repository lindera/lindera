//! Small wrappers of Node-API calls for the conversions that make
//! JavaScript values directly (`crate::token_objects`, `crate::batch_parts`).
//!
//! Each turns a failed status into an error. They are `unsafe`: every handle
//! must belong to the environment and the handle scope of the current call.

use std::ptr;

use napi::{check_status, sys};

/// The element of `array` at `index`.
///
/// # Safety
///
/// `env` must be the environment of the current call, and `array` an array
/// handle of it.
pub(crate) unsafe fn element(
    env: sys::napi_env,
    array: sys::napi_value,
    index: u32,
) -> napi::Result<sys::napi_value> {
    let mut value = ptr::null_mut();
    // SAFETY: the caller guarantees both handles; `value` outlives the call.
    check_status!(
        unsafe { sys::napi_get_element(env, array, index, &mut value) },
        "Failed to get an element of an array"
    )?;
    Ok(value)
}

/// Sets the element of `array` at `index`.
///
/// # Safety
///
/// As [`element`], and `value` must be a handle of the current call.
pub(crate) unsafe fn set_element(
    env: sys::napi_env,
    array: sys::napi_value,
    index: u32,
    value: sys::napi_value,
) -> napi::Result<()> {
    // SAFETY: the caller guarantees the three handles.
    check_status!(
        unsafe { sys::napi_set_element(env, array, index, value) },
        "Failed to set an element of an array"
    )
}

/// The length of `array`.
///
/// # Safety
///
/// As [`element`].
pub(crate) unsafe fn length(env: sys::napi_env, array: sys::napi_value) -> napi::Result<u32> {
    let mut len = 0;
    // SAFETY: the caller guarantees both handles; `len` outlives the call.
    check_status!(
        unsafe { sys::napi_get_array_length(env, array, &mut len) },
        "Failed to get the length of an array"
    )?;
    Ok(len)
}

/// The property `key` of `object`.
///
/// # Safety
///
/// `env` must be the environment of the current call, and `object` and
/// `key` handles of it.
pub(crate) unsafe fn property(
    env: sys::napi_env,
    object: sys::napi_value,
    key: sys::napi_value,
) -> napi::Result<sys::napi_value> {
    let mut value = ptr::null_mut();
    // SAFETY: the caller guarantees the handles; `value` outlives the call.
    check_status!(
        unsafe { sys::napi_get_property(env, object, key, &mut value) },
        "Failed to get a property of an object"
    )?;
    Ok(value)
}

/// A new array of `len` empty elements.
///
/// # Safety
///
/// `env` must be the environment of the current call.
pub(crate) unsafe fn array(env: sys::napi_env, len: usize) -> napi::Result<sys::napi_value> {
    let mut value = ptr::null_mut();
    // SAFETY: the caller guarantees `env`; `value` outlives the call.
    check_status!(
        unsafe { sys::napi_create_array_with_length(env, len, &mut value) },
        "Failed to create an array"
    )?;
    Ok(value)
}

/// A new string with the characters of `s`.
///
/// # Safety
///
/// `env` must be the environment of the current call.
pub(crate) unsafe fn string(env: sys::napi_env, s: &str) -> napi::Result<sys::napi_value> {
    let mut value = ptr::null_mut();
    // SAFETY: the caller guarantees `env`; `s` and `value` outlive the call,
    // and the length is that of `s` in bytes.
    check_status!(
        unsafe {
            sys::napi_create_string_utf8(env, s.as_ptr().cast(), s.len() as isize, &mut value)
        },
        "Failed to create a string"
    )?;
    Ok(value)
}

/// The number `n`, as napi-rs makes a `u32`.
///
/// # Safety
///
/// `env` must be the environment of the current call.
pub(crate) unsafe fn uint32(env: sys::napi_env, n: u32) -> napi::Result<sys::napi_value> {
    let mut value = ptr::null_mut();
    // SAFETY: the caller guarantees `env`; `value` outlives the call.
    check_status!(
        unsafe { sys::napi_create_uint32(env, n, &mut value) },
        "Failed to create a number"
    )?;
    Ok(value)
}

/// The number `n`, as napi-rs makes an `i64`.
///
/// # Safety
///
/// `env` must be the environment of the current call.
pub(crate) unsafe fn int64(env: sys::napi_env, n: i64) -> napi::Result<sys::napi_value> {
    let mut value = ptr::null_mut();
    // SAFETY: the caller guarantees `env`; `value` outlives the call.
    check_status!(
        unsafe { sys::napi_create_int64(env, n, &mut value) },
        "Failed to create a number"
    )?;
    Ok(value)
}

/// The boolean `b`.
///
/// # Safety
///
/// `env` must be the environment of the current call.
pub(crate) unsafe fn boolean(env: sys::napi_env, b: bool) -> napi::Result<sys::napi_value> {
    let mut value = ptr::null_mut();
    // SAFETY: the caller guarantees `env`; `value` outlives the call.
    check_status!(
        unsafe { sys::napi_get_boolean(env, b, &mut value) },
        "Failed to get a boolean"
    )?;
    Ok(value)
}

/// A new plain object with the properties of `names` and `values`, in that
/// order, each writable, enumerable and configurable: the object that
/// napi-rs makes of a `#[napi(object)]` struct.
///
/// # Safety
///
/// `env` must be the environment of the current call, and every name (a
/// string) and value a handle of it.
pub(crate) unsafe fn object<const N: usize>(
    env: sys::napi_env,
    names: &[sys::napi_value; N],
    values: [sys::napi_value; N],
) -> napi::Result<sys::napi_value> {
    let descriptors = std::array::from_fn::<_, N, _>(|i| sys::napi_property_descriptor {
        utf8name: ptr::null(),
        name: names[i],
        method: None,
        getter: None,
        setter: None,
        value: values[i],
        attributes: sys::PropertyAttributes::writable
            | sys::PropertyAttributes::enumerable
            | sys::PropertyAttributes::configurable,
        data: ptr::null_mut(),
    });
    let mut object = ptr::null_mut();
    // SAFETY: the caller guarantees the handles; `object` and the
    // descriptors outlive the calls.
    unsafe {
        check_status!(
            sys::napi_create_object(env, &mut object),
            "Failed to create an object"
        )?;
        check_status!(
            sys::napi_define_properties(env, object, N, descriptors.as_ptr()),
            "Failed to define the properties of an object"
        )?;
    }
    Ok(object)
}

/// Opens a handle scope.
///
/// # Safety
///
/// `env` must be the environment of the current call; the scope must be
/// closed with [`close_scope`], in the reverse order of opening.
pub(crate) unsafe fn open_scope(env: sys::napi_env) -> napi::Result<sys::napi_handle_scope> {
    let mut scope = ptr::null_mut();
    // SAFETY: the caller guarantees `env`; `scope` outlives the call.
    check_status!(
        unsafe { sys::napi_open_handle_scope(env, &mut scope) },
        "Failed to open a handle scope"
    )?;
    Ok(scope)
}

/// Closes a handle scope that [`open_scope`] opened.
///
/// # Safety
///
/// `scope` must be the innermost open scope of `env`; its handles are not
/// valid after this call.
pub(crate) unsafe fn close_scope(
    env: sys::napi_env,
    scope: sys::napi_handle_scope,
) -> napi::Result<()> {
    // SAFETY: the caller guarantees that `scope` is the innermost scope.
    check_status!(
        unsafe { sys::napi_close_handle_scope(env, scope) },
        "Failed to close a handle scope"
    )
}
