use reinhardt_macros::get;

struct Database;
struct Data;
struct Wrapper<T>(T);

fn borrow_mutably<T>(_: &mut T) {}

#[get("/mutable", use_inject = true)]
async fn mutable_handler(
	#[inject] mut db: Database,
	#[inject] Wrapper(mut value): Wrapper<Data>,
) -> String {
	borrow_mutably(&mut db);
	borrow_mutably(&mut value);
	String::new()
}

fn main() {}
