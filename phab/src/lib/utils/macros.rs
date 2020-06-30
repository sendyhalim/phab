// https://stackoverflow.com/questions/38088067/equivalent-of-func-or-function-in-rust
macro_rules! function_name {
  () => {{
    fn f() {}
    fn type_name_of<T>(_: T) -> &'static str {
      std::any::type_name::<T>()
    }
    let name = type_name_of(f);
    &name[..name.len() - 3]
  }};
}
