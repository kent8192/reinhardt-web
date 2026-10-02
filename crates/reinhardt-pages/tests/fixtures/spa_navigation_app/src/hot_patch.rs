use reinhardt_pages::page;

fn hot_patch_fixture() {
	let _view = page!(|| {
		div {
			id: "hot-patch-fixture",
			"Static template probe"
		}
	});
}
