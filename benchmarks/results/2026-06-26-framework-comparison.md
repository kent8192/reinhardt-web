# Framework Comparison Results - 2026-06-26

- Measured at: `2026-06-26 15:24:35 JST`
- Rust: `rustc 1.96.0 (ac68faa20 2026-05-25)`
- Cargo: `cargo 1.96.0 (30a34c682 2026-05-25)`
- Targets: Reinhardt, Axum, Actix Web, Loco
- Scenario coverage: 28 scenarios x 4 targets = 112 target measurements
- Lower values are better for every scenario.

## Methodology

- Runtime scenarios were measured with `cargo make benchmark-runtime-http`. The benchmark starts loopback HTTP servers for all targets and measures requests through one pooled `reqwest` client. Criterion config: `sample_size=10`, `warm_up_time=200ms`, `measurement_time=1s`.
- Non-runtime scenarios were measured with `cargo make benchmark-matrix-measure`. Database scenarios use the same in-memory SQLite fixture shape for all targets because Axum and Actix Web do not prescribe a database layer.
- Contract and admin scenarios use target-labeled native fixture adapters with identical route, row, and form shapes. Compile-time scenarios use generated temporary fixture crates under `/tmp`; the runner removes them through a Drop guard.
- Runtime values are Criterion slope point estimates with confidence intervals. Non-runtime values are arithmetic means with min/max over the recorded samples.

## Runtime HTTP Loopback

| Scenario | Unit | Reinhardt | Axum | Actix Web | Loco | Fastest |
| --- | --- | ---: | ---: | ---: | ---: | --- |
| `hello_world` | us/request | 45.274 us (41.203-48.161) | 40.988 us (38.062-47.344) | 43.366 us (40.800-45.789) | 38.048 us (36.018-38.942) | Loco |
| `json_echo` | us/request | 66.243 us (54.112-82.932) | 45.280 us (42.313-46.936) | 57.192 us (40.251-79.389) | 78.285 us (59.738-108.140) | Axum |
| `path_params` | us/request | 46.274 us (44.364-48.763) | 40.827 us (39.152-43.103) | 44.745 us (40.784-47.923) | 48.514 us (44.825-55.882) | Axum |
| `query_params` | us/request | 43.353 us (42.261-44.951) | 38.963 us (38.002-40.259) | 42.811 us (41.154-45.164) | 88.717 us (58.275-114.181) | Axum |
| `middleware_chain` | us/request | 41.077 us (38.188-43.737) | 40.337 us (38.279-43.531) | 38.855 us (37.147-41.839) | 55.589 us (47.262-72.331) | Actix Web |
| `dependency_injection` | us/request | 54.116 us (49.181-59.525) | 38.499 us (37.936-39.427) | 46.597 us (44.081-49.259) | 59.605 us (48.000-73.100) | Axum |
| `settings_access` | us/request | 49.251 us (44.807-52.786) | 39.354 us (38.483-40.897) | 41.360 us (40.713-42.527) | 58.240 us (48.153-66.284) | Axum |

## Non-Runtime Matrix

## database/single_select `query_latency`

| Target | Mean | Min | Max | Samples | Checksum |
| --- | ---: | ---: | ---: | ---: | ---: |
| Reinhardt | 3.975 us/query | 3.375 us/query | 21.333 us/query | 80 | 0 |
| Axum | 4.609 us/query | 3.500 us/query | 37.458 us/query | 80 | 0 |
| Actix Web | 8.131 us/query | 3.459 us/query | 172.667 us/query | 80 | 0 |
| Loco | 4.131 us/query | 3.458 us/query | 34.334 us/query | 80 | 0 |

## database/list_100_rows `query_latency`

| Target | Mean | Min | Max | Samples | Checksum |
| --- | ---: | ---: | ---: | ---: | ---: |
| Reinhardt | 72.879 us/query | 64.667 us/query | 181.625 us/query | 80 | 0 |
| Axum | 71.071 us/query | 63.791 us/query | 163.791 us/query | 80 | 0 |
| Actix Web | 77.693 us/query | 65.500 us/query | 117.708 us/query | 80 | 0 |
| Loco | 86.605 us/query | 62.875 us/query | 321.459 us/query | 80 | 0 |

## database/insert_one `mutation_latency`

| Target | Mean | Min | Max | Samples | Checksum |
| --- | ---: | ---: | ---: | ---: | ---: |
| Reinhardt | 7.059 us/mutation | 5.125 us/mutation | 43.750 us/mutation | 80 | 0 |
| Axum | 10.627 us/mutation | 5.041 us/mutation | 63.791 us/mutation | 80 | 0 |
| Actix Web | 5.336 us/mutation | 5.042 us/mutation | 11.125 us/mutation | 80 | 0 |
| Loco | 6.784 us/mutation | 5.000 us/mutation | 52.625 us/mutation | 80 | 0 |

## database/update_one `mutation_latency`

| Target | Mean | Min | Max | Samples | Checksum |
| --- | ---: | ---: | ---: | ---: | ---: |
| Reinhardt | 3.595 us/mutation | 3.000 us/mutation | 19.375 us/mutation | 80 | 0 |
| Axum | 3.542 us/mutation | 3.167 us/mutation | 7.250 us/mutation | 80 | 32 |
| Actix Web | 3.805 us/mutation | 3.333 us/mutation | 21.125 us/mutation | 80 | 32 |
| Loco | 8.163 us/mutation | 2.959 us/mutation | 110.041 us/mutation | 80 | 32 |

## database/transaction `transaction_latency`

| Target | Mean | Min | Max | Samples | Checksum |
| --- | ---: | ---: | ---: | ---: | ---: |
| Reinhardt | 104.293 us/transaction | 51.125 us/transaction | 756.875 us/transaction | 80 | 0 |
| Axum | 72.419 us/transaction | 48.834 us/transaction | 1526.333 us/transaction | 80 | 0 |
| Actix Web | 55.543 us/transaction | 52.750 us/transaction | 77.250 us/transaction | 80 | 0 |
| Loco | 164.679 us/transaction | 49.750 us/transaction | 452.292 us/transaction | 80 | 0 |

## database/n_plus_one_detection `analysis_latency`

| Target | Mean | Min | Max | Samples | Checksum |
| --- | ---: | ---: | ---: | ---: | ---: |
| Reinhardt | 19.644 us/check | 17.209 us/check | 59.334 us/check | 30 | 0 |
| Axum | 21.737 us/check | 14.625 us/check | 67.583 us/check | 30 | 0 |
| Actix Web | 17.674 us/check | 15.625 us/check | 58.958 us/check | 30 | 0 |
| Loco | 16.587 us/check | 15.458 us/check | 32.917 us/check | 30 | 0 |

## compile_time/clean_build_minimal `wall_clock_time`

| Target | Mean | Min | Max | Samples | Checksum |
| --- | ---: | ---: | ---: | ---: | ---: |
| Reinhardt | 64.566 s/build | 64.566 s/build | 64.566 s/build | 1 | 2055013176433237210 |
| Axum | 12.208 s/build | 12.208 s/build | 12.208 s/build | 1 | 11634090401551755282 |
| Actix Web | 42.336 s/build | 42.336 s/build | 42.336 s/build | 1 | 11131896852758035393 |
| Loco | 92.679 s/build | 92.679 s/build | 92.679 s/build | 1 | 13763599105740245446 |

## compile_time/clean_build_full `wall_clock_time`

| Target | Mean | Min | Max | Samples | Checksum |
| --- | ---: | ---: | ---: | ---: | ---: |
| Reinhardt | 89.444 s/build | 89.444 s/build | 89.444 s/build | 1 | 5106412299726589338 |
| Axum | 16.013 s/build | 16.013 s/build | 16.013 s/build | 1 | 4718597423864353506 |
| Actix Web | 39.823 s/build | 39.823 s/build | 39.823 s/build | 1 | 5381712491456488055 |
| Loco | 113.903 s/build | 113.903 s/build | 113.903 s/build | 1 | 4960296255313796782 |

## compile_time/incremental_model_change `wall_clock_time`

| Target | Mean | Min | Max | Samples | Checksum |
| --- | ---: | ---: | ---: | ---: | ---: |
| Reinhardt | 0.405 s/build | 0.405 s/build | 0.405 s/build | 1 | 16485730096074290523 |
| Axum | 0.152 s/build | 0.152 s/build | 0.152 s/build | 1 | 10154544799070714819 |
| Actix Web | 0.211 s/build | 0.211 s/build | 0.211 s/build | 1 | 11821608666574591714 |
| Loco | 0.417 s/build | 0.417 s/build | 0.417 s/build | 1 | 4658543231903666167 |

## compile_time/incremental_route_change `wall_clock_time`

| Target | Mean | Min | Max | Samples | Checksum |
| --- | ---: | ---: | ---: | ---: | ---: |
| Reinhardt | 0.505 s/build | 0.505 s/build | 0.505 s/build | 1 | 16009292086125322551 |
| Axum | 0.216 s/build | 0.216 s/build | 0.216 s/build | 1 | 10348889012862544863 |
| Actix Web | 0.706 s/build | 0.706 s/build | 0.706 s/build | 1 | 5207754521253649442 |
| Loco | 0.598 s/build | 0.598 s/build | 0.598 s/build | 1 | 18084709820350518203 |

## compile_time/cargo_check `wall_clock_time`

| Target | Mean | Min | Max | Samples | Checksum |
| --- | ---: | ---: | ---: | ---: | ---: |
| Reinhardt | 0.277 s/check | 0.277 s/check | 0.277 s/check | 1 | 14174857574394537035 |
| Axum | 0.050 s/check | 0.050 s/check | 0.050 s/check | 1 | 16681283847060340547 |
| Actix Web | 0.136 s/check | 0.136 s/check | 0.136 s/check | 1 | 2024873457742838084 |
| Loco | 0.340 s/check | 0.340 s/check | 0.340 s/check | 1 | 6686376239281038919 |

## contract/introspect_small_app `introspection_latency`

| Target | Mean | Min | Max | Samples | Checksum |
| --- | ---: | ---: | ---: | ---: | ---: |
| Reinhardt | 67.545 us/run | 65.666 us/run | 100.291 us/run | 50 | 0 |
| Axum | 65.324 us/run | 62.667 us/run | 84.208 us/run | 50 | 0 |
| Actix Web | 67.848 us/run | 65.333 us/run | 103.167 us/run | 50 | 0 |
| Loco | 63.798 us/run | 62.458 us/run | 70.292 us/run | 50 | 0 |

## contract/introspect_medium_app `introspection_latency`

| Target | Mean | Min | Max | Samples | Checksum |
| --- | ---: | ---: | ---: | ---: | ---: |
| Reinhardt | 584.826 us/run | 547.375 us/run | 638.917 us/run | 30 | 0 |
| Axum | 552.412 us/run | 520.875 us/run | 611.917 us/run | 30 | 0 |
| Actix Web | 569.583 us/run | 542.000 us/run | 684.167 us/run | 30 | 0 |
| Loco | 543.804 us/run | 515.417 us/run | 724.500 us/run | 30 | 0 |

## contract/validate_contract `validation_latency`

| Target | Mean | Min | Max | Samples | Checksum |
| --- | ---: | ---: | ---: | ---: | ---: |
| Reinhardt | 591.115 us/run | 555.916 us/run | 754.125 us/run | 50 | 0 |
| Axum | 558.108 us/run | 530.667 us/run | 696.834 us/run | 50 | 0 |
| Actix Web | 598.185 us/run | 562.542 us/run | 744.208 us/run | 50 | 0 |
| Loco | 545.370 us/run | 529.709 us/run | 611.250 us/run | 50 | 0 |

## contract/generate_cloud_plan `plan_generation_latency`

| Target | Mean | Min | Max | Samples | Checksum |
| --- | ---: | ---: | ---: | ---: | ---: |
| Reinhardt | 16.606 us/run | 16.041 us/run | 30.042 us/run | 50 | 0 |
| Axum | 15.250 us/run | 15.000 us/run | 15.625 us/run | 50 | 0 |
| Actix Web | 16.364 us/run | 16.125 us/run | 16.833 us/run | 50 | 0 |
| Loco | 15.372 us/run | 15.083 us/run | 16.250 us/run | 50 | 0 |

## contract/dry_run_deploy `dry_run_latency`

| Target | Mean | Min | Max | Samples | Checksum |
| --- | ---: | ---: | ---: | ---: | ---: |
| Reinhardt | 16.968 us/run | 16.167 us/run | 22.000 us/run | 50 | 9664707209161652 |
| Axum | 13.225 us/run | 12.125 us/run | 36.458 us/run | 50 | 28600496495260948 |
| Actix Web | 16.428 us/run | 15.291 us/run | 22.208 us/run | 50 | 4846647255563732 |
| Loco | 13.143 us/run | 12.292 us/run | 16.708 us/run | 50 | 2363949999981436 |

## admin/list_view_1k `render_latency`

| Target | Mean | Min | Max | Samples | Checksum |
| --- | ---: | ---: | ---: | ---: | ---: |
| Reinhardt | 65.227 us/render | 63.334 us/render | 83.417 us/render | 30 | 0 |
| Axum | 48.825 us/render | 45.709 us/render | 88.333 us/render | 30 | 0 |
| Actix Web | 63.045 us/render | 62.667 us/render | 65.541 us/render | 30 | 0 |
| Loco | 46.050 us/render | 45.625 us/render | 48.750 us/render | 30 | 0 |

## admin/list_view_100k `render_latency`

| Target | Mean | Min | Max | Samples | Checksum |
| --- | ---: | ---: | ---: | ---: | ---: |
| Reinhardt | 6497.489 us/render | 6408.292 us/render | 6592.625 us/render | 8 | 0 |
| Axum | 4669.432 us/render | 4608.125 us/render | 4787.834 us/render | 8 | 0 |
| Actix Web | 6463.062 us/render | 6376.666 us/render | 6565.208 us/render | 8 | 0 |
| Loco | 4731.041 us/render | 4678.875 us/render | 4848.333 us/render | 8 | 0 |

## admin/detail_view `render_latency`

| Target | Mean | Min | Max | Samples | Checksum |
| --- | ---: | ---: | ---: | ---: | ---: |
| Reinhardt | 5.640 us/render | 5.125 us/render | 15.250 us/render | 30 | 17627663416339381211 |
| Axum | 5.190 us/render | 4.833 us/render | 6.542 us/render | 30 | 5578074003081965397 |
| Actix Web | 5.401 us/render | 5.083 us/render | 6.667 us/render | 30 | 15920650086714534253 |
| Loco | 4.952 us/render | 4.791 us/render | 5.792 us/render | 30 | 13707026094407340561 |

## admin/create_form `form_latency`

| Target | Mean | Min | Max | Samples | Checksum |
| --- | ---: | ---: | ---: | ---: | ---: |
| Reinhardt | 2.359 us/render | 2.000 us/render | 6.333 us/render | 30 | 0 |
| Axum | 2.248 us/render | 1.958 us/render | 2.709 us/render | 30 | 0 |
| Actix Web | 2.173 us/render | 2.042 us/render | 2.875 us/render | 30 | 0 |
| Loco | 2.080 us/render | 2.000 us/render | 2.708 us/render | 30 | 0 |

## admin/search_filter `search_latency`

| Target | Mean | Min | Max | Samples | Checksum |
| --- | ---: | ---: | ---: | ---: | ---: |
| Reinhardt | 59.047 us/search | 57.792 us/search | 66.166 us/search | 30 | 320 |
| Axum | 58.773 us/search | 56.583 us/search | 72.625 us/search | 30 | 15496 |
| Actix Web | 59.793 us/search | 58.334 us/search | 63.584 us/search | 30 | 872 |
| Loco | 58.439 us/search | 57.250 us/search | 66.333 us/search | 30 | 320 |
