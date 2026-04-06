if switch_enabled("test_runtime_error") {
    let x = 1 / 0;
}

if switch_enabled("test_undefined_function") {
    nonexistent_function();
}
