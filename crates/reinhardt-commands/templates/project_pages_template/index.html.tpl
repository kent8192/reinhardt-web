<!-- WASM initialization is handled automatically by StaticFilesMiddleware (rc.15+).
     Do not add manual wasm-bindgen init scripts here. -->
<!DOCTYPE html>
<html lang="en">
<head>
	<meta charset="UTF-8">
	<meta name="viewport" content="width=device-width, initial-scale=1.0">
	<title>{{ project_name }} - Reinhardt Pages</title>
	<link
		href="https://cdn.jsdelivr.net/npm/bootstrap@5.3.8/dist/css/bootstrap.min.css"
		rel="stylesheet"
		integrity="sha384-sRIl4kxILFvY47J16cr9ZwB07vP4J8+LH7qKQnuqkuIAvNWLzeN8tE5YBujZqJLB"
		crossorigin="anonymous">
	<style>
		body {
			background-color: #f8f9fa;
		}
		.loading {
			display: flex;
			justify-content: center;
			align-items: center;
			height: 100vh;
		}
	</style>
</head>
<body>
	<div id="root">
		<div class="loading">
			<div class="spinner-border text-primary" role="status">
				<span class="visually-hidden">Loading...</span>
			</div>
		</div>
	</div>
</body>
</html>
