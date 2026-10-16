# Migration Guide: 0.3.x to 0.3.24

0.3.24 includes an intentional breaking security correction for `QuerySet`
projections and identifier joins despite retaining the 0.3 release line. This
is the scoped exception in
[ST-3](STABILITY_POLICY.md#st-3-scoped-security-exception-for-0324) and fixes
[GHSA-6rxp-cc74-26f5](https://github.com/kent8192/reinhardt-web/security/advisories/GHSA-6rxp-cc74-26f5).
Cargo requirements such as `"0.3.23"` permit 0.3.24, so an ordinary
dependency update can select this release.

## Why behavior changes

`QuerySet::values`, `values_list`, and `only` emitted any entry containing
parentheses as raw SQL. `inner_join`, `left_join`, and `right_join`
interpolated their field arguments into a raw `ON` condition, and the
`*_join_as` methods did the same with table aliases. Applications that derive
those strings from request data, for example a field-selection parameter,
let an attacker run SQL expressions or widen a join predicate. These methods
take plain strings, so their signatures cannot distinguish a trusted expression
from attacker input; every entry is now treated as an identifier, except a
fixed set of aggregates that are rendered as typed expressions.

## Required migration

1. Replace expression projections passed to `values`, `values_list`, or
   `only`. Only `COUNT(*)` and `COUNT`, `SUM`, `AVG`, `MIN`, or `MAX` over a
   single field name are still rendered as aggregates, with the field quoted.
   Any other entry containing parentheses, such as `LOWER(email)`, is rendered
   by `to_sql` as one quoted identifier, and executing the queryset returns
   `Error::Validation`. Use `aggregate` or `annotate` for other computed
   columns. For example, a count subquery becomes:

   ```rust,ignore
   use reinhardt_db::orm::aggregation::Aggregate;

   let authors = Author::objects()
       .annotate_subquery::<Book, _>("book_count", |subq| {
           subq.filter("author_id", FilterOperator::Eq, FilterValue::OuterRef(OuterRef::new("authors.id")))
               .values(&[])
               .aggregate(Aggregate::count_all().with_alias("count"))
       })
       .all()
       .await?;
   ```

2. Pass plain column names to `inner_join`, `left_join`, and `right_join`.
   Each argument becomes one quoted column qualified by its model's table
   name, so pre-quoted names, dotted paths, and expressions no longer form
   part of the `ON` clause. For a trusted custom condition, use
   `inner_join_on`, `left_join_on`, or `right_join_on` with a fixed SQL string
   that never contains request data.
3. Treat aliases passed to `inner_join_as`, `left_join_as`, and
   `right_join_as` as quoted, case-sensitive identifiers. On PostgreSQL an
   alias such as `U1` no longer folds to `u1` in the `ON` condition.

## Verify the upgrade

Search for affected calls and exercise every query built from them:

```bash
rg -n '\.(values|values_list|only)\(|\.(inner|left|right)_join(_as)?(::<|\()' src crates
```

Confirm that projections contain only field names or the supported aggregates,
that join arguments are column names, and that queries built from request data
still reject unknown fields at the application layer. An API-compatible
`cargo-semver-checks` result alone does not verify these runtime contracts.

This exception is limited to the `QuerySet` projection and join behaviors
described above. Other APIs and later releases retain the normal 0.3
compatibility policy.
