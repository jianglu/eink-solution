use winreg::{RegKey, enums::HKEY_LOCAL_MACHINE};


/// 服务助手程序
fn main() -> anyhow::Result<()> {

    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);

    let reg_key = hklm
        .open_subkey(r#"SOFTWARE\Lenovo\ThinkBookEinkPlus"#);

    // reg_key: Err(Os { code: 2, kind: NotFound, message: "The system cannot find the file specified." })
    println!("reg_key: {reg_key:?}");

    let reg_key = reg_key.unwrap();

    // Error: The system cannot find the file specified. (os error 2)
    let value = reg_key.get_value::<String, &str>("DisplayValue").unwrap_err();
    println!("value: {value:?}");

    Ok(())
}
