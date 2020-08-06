#![deny(rust_2018_idioms, unused, unused_import_braces, unused_qualifications, warnings)]

use {
    pyo3::{
        create_exception_type_object,
        impl_exception_boilerplate,
        prelude::*,
        wrap_pyfunction
    },
    serenity::{
        model::prelude::*,
        utils::MessageBuilder
    }
};

struct CommandError;

impl_exception_boilerplate!(CommandError);

fn user_to_id(user: &PyAny) -> PyResult<UserId> {
    if let Ok(snowflake) = user.getattr("snowflake") {
        // support gefolge_web.login.Mensch arguments
        Ok(UserId(snowflake.extract()?))
    } else {
        // support plain snowflakes
        Ok(UserId(user.extract()?))
    }
}

#[pyfunction] fn escape(text: &str) -> String {
    let mut builder = MessageBuilder::default();
    builder.push_safe(text);
    builder.build()
}

#[pyfunction] fn add_role(user_id: &PyAny, role_id: u64) -> PyResult<()> {
    peter_ipc::add_role(user_to_id(user_id)?, RoleId(role_id))
        .map_err(|e| CommandError::py_err(e.to_string()))
}

#[pyfunction] fn channel_msg(channel_id: u64, msg: String) -> PyResult<()> {
    peter_ipc::channel_msg(ChannelId(channel_id), msg)
        .map_err(|e| CommandError::py_err(e.to_string()))
}

#[pyfunction] fn msg(user_id: &PyAny, msg: String) -> PyResult<()> {
    peter_ipc::msg(user_to_id(user_id)?, msg)
        .map_err(|e| CommandError::py_err(e.to_string()))
}

#[pyfunction] fn quit() -> PyResult<()> {
    peter_ipc::quit()
        .map_err(|e| CommandError::py_err(e.to_string()))
}

#[pyfunction] fn set_display_name(user_id: &PyAny, new_display_name: String) -> PyResult<()> {
    peter_ipc::set_display_name(user_to_id(user_id)?, new_display_name)
        .map_err(|e| CommandError::py_err(e.to_string()))
}

#[pymodule] fn peter(_: Python<'_>, m: &PyModule) -> PyResult<()> {
    create_exception_type_object!(m, CommandError, pyo3::exceptions::RuntimeError);
    m.add_wrapped(wrap_pyfunction!(escape))?;
    //TODO make sure that all IPC commands are listed below
    m.add_wrapped(wrap_pyfunction!(add_role))?;
    m.add_wrapped(wrap_pyfunction!(channel_msg))?;
    m.add_wrapped(wrap_pyfunction!(msg))?;
    m.add_wrapped(wrap_pyfunction!(quit))?;
    m.add_wrapped(wrap_pyfunction!(set_display_name))?;
    Ok(())
}
