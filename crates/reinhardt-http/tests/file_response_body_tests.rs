use reinhardt_http::Response;
use rstest::rstest;
use std::io::Write;

#[rstest]
fn cloned_file_bodies_have_independent_read_positions_and_bounded_ranges() {
	// Arrange
	let mut source = tempfile::tempfile().unwrap();
	source.write_all(b"0123456789").unwrap();
	let response = Response::ok().with_file_body(source, 2, 5).unwrap();
	let cloned = response.clone();
	// Act
	let first = response.file_body().unwrap().read_chunk(0, 2).unwrap();
	let later = cloned.file_body().unwrap().read_chunk(3, 64).unwrap();
	let repeated = response.file_body().unwrap().read_chunk(0, 2).unwrap();
	// Assert
	assert_eq!(first.as_ref(), b"23");
	assert_eq!(later.as_ref(), b"56");
	assert_eq!(repeated, first);
	assert!(response.body.is_empty());
	assert_eq!(response.file_body().unwrap().len(), 5);
	assert!(
		response
			.file_body()
			.unwrap()
			.read_chunk(5, 64)
			.unwrap()
			.is_empty()
	);
	assert!(response.with_body("replacement").file_body().is_none());
}

#[rstest]
fn sparse_large_files_are_read_in_small_chunks() {
	// Arrange
	let source = tempfile::tempfile().unwrap();
	let length = 4 * 1024 * 1024 * 1024_u64;
	source.set_len(length).unwrap();
	// Act
	let response = Response::ok().with_file_body(source, 0, length).unwrap();
	let chunk = response
		.file_body()
		.unwrap()
		.read_chunk(length - 32, 1024)
		.unwrap();
	// Assert
	assert_eq!(chunk.as_ref(), &[0; 32]);
	assert!(response.body.is_empty());
	assert_eq!(response.file_body().unwrap().len(), length);
}

#[rstest]
#[case(4, 2)]
#[case(u64::MAX, 2)]
fn file_ranges_outside_the_captured_file_are_rejected(#[case] offset: u64, #[case] length: u64) {
	// Arrange
	let source = tempfile::tempfile().unwrap();
	source.set_len(5).unwrap();
	// Act
	let result = Response::ok().with_file_body(source, offset, length);
	// Assert
	assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::InvalidInput);
}

#[cfg(unix)]
#[rstest]
fn duplicated_handles_cannot_race_another_response_cursor() {
	// Arrange
	let mut file = tempfile::tempfile().unwrap();
	file.write_all(b"0123456789").unwrap();
	let left = Response::ok()
		.with_file_body(file.try_clone().unwrap(), 0, 5)
		.unwrap();
	let right = Response::ok().with_file_body(file, 5, 5).unwrap();
	// Act & Assert
	std::thread::scope(|scope| {
		scope.spawn(|| {
			for _ in 0..1000 {
				assert_eq!(
					left.file_body().unwrap().read_chunk(0, 5).unwrap().as_ref(),
					b"01234"
				);
			}
		});
		scope.spawn(|| {
			for _ in 0..1000 {
				assert_eq!(
					right
						.file_body()
						.unwrap()
						.read_chunk(0, 5)
						.unwrap()
						.as_ref(),
					b"56789"
				);
			}
		});
	});
}
