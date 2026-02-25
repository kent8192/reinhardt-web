//! page! macro with deep element nesting
//!
//! This test verifies that the page! macro can handle deeply nested elements
//! without issues (boundary value testing for nesting depth).

use reinhardt_pages::page;

fn main() {
	// 10 levels of nesting
	let _deep_nesting_10 = page!(|| {
		div {
			div {
				div {
					div {
						div {
							div {
								div {
									div {
										div {
											div {
												"10 levels deep"
											}
										}
									}
								}
							}
						}
					}
				}
			}
		}
	});

	// 15 levels with mixed elements
	let _deep_nesting_15 = page!(|| {
		section {
			article {
				div {
					ul {
						li {
							div {
								span {
									strong {
										em {
											code {
												pre {
													blockquote {
														p {
															a {
																href: "#",
																span {
																	"15 levels deep"
																}
															}
														}
													}
												}
											}
										}
									}
								}
							}
						}
					}
				}
			}
		}
	});

	// 20 levels (extreme case)
	let _extreme_nesting = page!(|| {
		div {
			div {
				div {
					div {
						div {
							div {
								div {
									div {
										div {
											div {
												div {
													div {
														div {
															div {
																div {
																	div {
																		div {
																			div {
																				div {
																					div {
																						"Extremely deep"
																					}
																				}
																			}
																		}
																	}
																}
															}
														}
													}
												}
											}
										}
									}
								}
							}
						}
					}
				}
			}
		}
	});
}
