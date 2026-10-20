+++
title = "Cookbook"
description = "Practical recipes for common Reinhardt patterns."
sort_by = "weight"
weight = 50

[extra]
sidebar_weight = 50
+++


# Cookbook

Practical tutorials and recipes for the Reinhardt framework.

## Table of Contents

- [Serving Static Files](static-files/)
- [CORS Configuration](cors/)
- [Middleware Creation](middleware-creation/)
- [Pagination](pagination/)
- [Request Body Parsing](request-body-parsing/)
- [Response Serialization](response-serialization/)
- [Request Admission](request-admission/)

## Overview

The Cookbook provides practical guides for solving common tasks.

### Documentation Structure

- **Serving Static Files**: Storage backends, cache strategies, development vs production differences
- **CORS Configuration**: Preflight requests, origin restrictions, credentials handling
- **Middleware Creation**: Custom middleware implementation, state management, conditional execution
- **Pagination**: Cursor-based and offset-based pagination strategies
- **Request Body Parsing**: JSON, form data, multipart, and raw body extraction
- **Response Serialization**: JSON responses, custom serialization, streaming
- **Request Admission**: Body budgets, receive timeouts, and overload rejection before the body is read, in deployment and under `runserver`

## See Also

<!-- reinhardt-version-sync -->
- [API Reference](https://docs.rs/reinhardt-web/0.4.0-alpha.22/reinhardt/)
- [Migration Guides](/quickstart/migration-guides/)
- [Troubleshooting](../troubleshooting/)
