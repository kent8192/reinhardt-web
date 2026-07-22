use reinhardt_macros::routes;

struct Database;
struct Data;
struct Wrapper<T>(T);
struct UnifiedRouter;

fn borrow_mutably<T>(_: &mut T) {}

#[routes]
async fn mutable_routes(
	#[inject] mut db: Database,
	#[inject] Wrapper(mut value): Wrapper<Data>,
) -> UnifiedRouter {
	borrow_mutably(&mut db);
	borrow_mutably(&mut value);
	UnifiedRouter
}

fn main() {}
