mod tools;

fn main() {
    let a = tools::Functions::new("./".into()).unwrap();
    print!("{:?}", a.get_all_files().unwrap())
}
