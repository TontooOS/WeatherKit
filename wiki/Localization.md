# Localization

WeatherKit ships built-in language files for all error messages, following
the same scheme as CoreLocation and NetworkKit. The files live in
`lang/en_us.json` and `lang/de_de.json` and are compiled into the binary with
`include_str!`.

## Message Keys

| Key | Error variant | en_us text |
|---|---|---|
| `not_available` | `WeatherError::NotAvailable` | No weather source reachable |
| `permission_denied` | `WeatherError::PermissionDenied` | Permission denied |
| `timeout` | `WeatherError::Timeout` | Request timed out |
| `network_error` | `WeatherError::NetworkError` | Network error: {} |
| `parse_error` | `WeatherError::ParseError` | Parse error: {} |
| `provider_failed` | `WeatherError::ProviderFailed` | Provider failed: {} |
| `location_failed` | `WeatherError::LocationFailed` | Could not determine location: {} |

## Locale Detection

```rust
pub fn current_locale() -> &'static str
```

Checks the environment variables `LC_ALL`, `LC_MESSAGES` and `LANG` in this
order. A value starting with `de` selects `de_de`, everything else falls back
to `en_us`. Detection happens once; the parsed message table is cached in a
static.

## Accessing Messages

```rust
pub fn t(key: &str) -> String
pub fn t_fmt(key: &str, arg: &str) -> String
```

- `t` returns the translated message or the key itself when unknown.
- `t_fmt` replaces the first `{}` placeholder with the argument.
- `Display` for every `WeatherError` variant uses these functions, so
  `format!("{}", err)` is always localized.
- Fallback errors combine both failures: `"Provider failed: <primary> and
  fallback failed (<fallback>)"`.

## Usage / Example

```rust
use weatherkit::{lang, WeatherKit};

let kit = WeatherKit::new();
if let Err(err) = kit.current_weather() {
    println!("{}", err);
}

assert_eq!(lang::t("not_available"), "No weather source reachable");
```

## Cross References

- [MAIN.md](MAIN.md) – feature index
