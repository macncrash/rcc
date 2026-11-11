// env::var / var_os with variables provided by t11_env_var.env, plus an unset one
use std::env;

fn main() {
    match env::var("RCC_TEST_GREETING") {
        Ok(v) => println!("greeting = {}", v),
        Err(e) => println!("greeting error {:?}", e),
    }
    match env::var("RCC_TEST_UNSET_VAR_XYZ") {
        Ok(v) => println!("unset = {}", v),
        Err(env::VarError::NotPresent) => println!("unset: not present"),
        Err(e) => println!("unset other error {:?}", e),
    }
    let n: i32 = env::var("RCC_TEST_NUMBER").ok().and_then(|s| s.parse().ok()).unwrap_or(-1);
    println!("number = {} doubled {}", n, n * 2);
    let empty = env::var("RCC_TEST_EMPTY");
    println!("empty is_ok {} value [{}]", empty.is_ok(), empty.unwrap_or_default());
    println!("var_os some {}", env::var_os("RCC_TEST_GREETING").is_some());
    println!("var_os unset none {}", env::var_os("RCC_TEST_UNSET_VAR_XYZ").is_none());
    let list = env::var("RCC_TEST_LIST").unwrap_or_default();
    for (i, part) in list.split(':').enumerate() {
        println!("list[{}] = {}", i, part);
    }
    let mut mine: Vec<(String, String)> = env::vars().filter(|(k, _)| k.starts_with("RCC_TEST_")).collect();
    mine.sort();
    for (k, v) in &mine {
        println!("vars: {}={}", k, v);
    }
    println!("default via unwrap_or {}", env::var("RCC_TEST_UNSET_VAR_XYZ").unwrap_or_else(|_| "fallback".to_string()));
}
