# Weaver Forge - A Jinja-based Doc/Code Generation Engine

## Table of Contents

- [Introduction](#introduction)
- [General Concepts](#general-concepts)
    - [Template Directory Structure and Naming Conventions](#template-directory-structure-and-naming-conventions)
    - [Configuration File - `weaver.yaml`](#configuration-file---weaveryaml)
    - [Global Variables](#global-variables)
    - [JQ Filters](#jq-filters)
- [Step-by-Step Guide](#step-by-step-guide)
    - [Step 1: Setting Up Your Template Directory](#step-1-setting-up-your-template-directory)
    - [Step 2: Creating and Configuring `weaver.yaml`](#step-2-creating-and-configuring-weaveryaml)
    - [Step 3: Writing Your First Template](#step-3-writing-your-first-template)
- [In-Depth Features](#in-depth-features)
    - [JQ Filters Reference](#jq-filters-reference)
    - [Jinja Filters Reference](#jinja-filters-reference)
    - [Jinja Functions Reference](#jinja-functions-reference)
    - [Jinja Tests Reference](#jinja-tests-reference)

## Introduction

Weaver Forge is a component of OTEL Weaver that facilitates documentation and
code generation from a semantic convention registry. It uses MiniJinja, a
template engine compatible with Jinja2 syntax, which provides extensive
customization options (refer to this [GitHub repository](https://github.com/mitsuhiko/minijinja)
for more details). Some good references to start developing Jinja2 templages are 
[1](https://ttl255.com/jinja2-tutorial-part-2-loops-and-conditionals/) and 
[2](https://jinja.palletsprojects.com/en/stable/templates).
To streamline template creation for semantic conventions,
additional filters, functions, tests, and naming conventions have been
integrated with the standard Jinja logic.

Weaver Forge also incorporates a YAML/JSON processor compatible with JQ to
preprocess resolved registries before they are processed by Jinja templates.
This integration helps avoid complex logic within the templates. A set of
specialized JQ filters is available to extract and organize attributes and
metrics, making them directly usable by the templates. This allows
template authors to focus on rendering rather than filtering, transforming, or
ordering logic in Jinja.

The following diagram illustrates the documentation and code generation pipeline using the OTEL
Weaver tool:

![Weaver Forge](images/artifact-generation-pipeline.svg)

Weaver's resolution process simplifies the semantic conventions by eliminating
references, extend statements, and other complex constructs, creating a fully
resolved, easy-to-use, self-contained version of the registry. This resolved
registry can be optionally filtered, grouped, sorted, and processed using a
JQ-based transformation before being used by the Jinja-based template engine
for documentation and code generation. Additionally, a set of templates and a
configuration file, stored alongside these templates, are processed by the
template engine to generate the desired artifacts.

## General Concepts

### Template Directory Structure and Naming Conventions

By default, Weaver looks for a directory named `templates/`, which contains
several collection of templates, also referred to as targets (e.g. go, html,
markdown, rust, ...). The hierarchical structure of the `templates` directory
is detailed below. Note that this location can be changed using the `-t` or
`--templates` CLI parameter.

```plaintext
templates/
  registry/
    go/
      ...
    html/
      ...
    markdown/
      ...
    rust/
      ...
    .../
``` 