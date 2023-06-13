# wasm-faas

Plan for efficient multi-tenant web platform

## Build off of core wasm creates

Use core wasm crate and manage the modules lifetime and state manually.

Modules just need a function that accepts JSON and returns JSON.

Can automatically load modules when files added to a certain dir.

Provide microkv with caching for modules.

Create a scheeme similar to row-level-security for microkv to allow user
data records to be protected from modification by other users or certain
types of records to only be modified by admins.

Serve pre-compiled YEW via actix or something.

YEW theme system? Fill in with JSON objects?

https://blog.scottlogic.com/2022/04/16/wasm-faas.html

## OpenAPI for managing data, themes, etc.?

Needs to be accessible by AI and also from within
site admin by user


