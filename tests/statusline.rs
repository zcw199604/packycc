use ccometixline::config::{Config, InputData};
use ccometixline::core::StatusLineGenerator;
use serde_json::{json, Value};

fn render(fields: Value) -> String {
    let mut input = json!({
        "model": { "display_name": "Sonnet 4.5" },
        "workspace": { "current_dir": "/example" },
        "transcript_path": "/example/session.jsonl"
    });
    input
        .as_object_mut()
        .unwrap()
        .extend(fields.as_object().unwrap().clone());
    let input: InputData = serde_json::from_value(input).unwrap();
    let mut config = Config::default();
    config.segments.directory = false;
    config.segments.git = false;
    config.segments.cost = false;
    StatusLineGenerator::new(config).generate_with_width(&input, None)
}

fn strip_ansi(s: &str) -> String {
    let mut out = String::new();
    let mut in_escape = false;
    for c in s.chars() {
        match c {
            '\x1b' => in_escape = true,
            'm' if in_escape => in_escape = false,
            _ if !in_escape => out.push(c),
            _ => {}
        }
    }
    out
}

fn context(input_tokens: u32, cache_creation: u32, cache_read: u32) -> Value {
    json!({
        "context_window_size": 200_000,
        "current_usage": {
            "input_tokens": input_tokens,
            "output_tokens": 1200,
            "cache_creation_input_tokens": cache_creation,
            "cache_read_input_tokens": cache_read
        }
    })
}

#[test]
fn live_effort_follows_model_and_session_cache_follows_context() {
    let output = render(json!({
        "effort": { "level": "high" },
        "prompt_cache": { "hit_ratio": 0.91 },
        "context_window": context(8500, 5000, 2000)
    }));

    assert!(output.contains("Sonnet 4.5 High"), "{output}");
    assert!(!output.contains("[high]"), "{output}");
    assert!(output.contains("7.75% (15.50K/200.00K)"), "{output}");
    assert!(output.contains("Cache 91.00%"), "{output}");
    assert!(output.find("15.50K/200.00K").unwrap() < output.find("Cache 91.00%").unwrap());
    assert!(!output.contains("Cache(last)"), "{output}");
}

#[test]
fn older_clients_show_last_request_cache_fraction_with_all_input_categories() {
    let output = render(json!({ "context_window": context(8500, 5000, 2000) }));

    assert!(output.contains("Cache(last) 12.90%"), "{output}");
    assert!(!output.contains("High"), "{output}");
}

#[test]
fn null_session_ratio_falls_back_to_last_request() {
    let output = render(json!({
        "prompt_cache": { "hit_ratio": null },
        "context_window": context(1000, 1000, 8000)
    }));

    assert!(output.contains("Cache(last) 80.00%"), "{output}");
}

#[test]
fn zero_session_hit_ratio_is_not_replaced_with_last_request_ratio() {
    let output = render(json!({
        "prompt_cache": { "hit_ratio": 0.0 },
        "context_window": context(0, 0, 8000)
    }));

    assert!(output.contains("Cache 0.00%"), "{output}");
    assert!(!output.contains("100.00%"), "{output}");
}

#[test]
fn unknown_cache_and_effort_are_hidden_including_before_first_response() {
    for fields in [
        json!({}),
        json!({ "effort": null, "prompt_cache": null, "context_window": null }),
        json!({ "context_window": { "current_usage": null } }),
        json!({ "context_window": context(0, 0, 0) }),
    ] {
        let output = render(fields);
        assert!(output.contains("Sonnet 4.5"), "{output}");
        assert!(!output.contains("Cache"), "{output}");
        assert!(!output.contains("High"), "{output}");
    }
}

#[test]
fn session_cache_survives_context_usage_being_cleared_after_compaction() {
    let output = render(json!({
        "prompt_cache": { "hit_ratio": 0.91 },
        "context_window": { "context_window_size": 200_000, "current_usage": null }
    }));

    assert!(output.contains("Cache 91.00%"), "{output}");
    assert!(!output.contains("▓"), "{output}");
}

#[test]
fn subscription_quota_follows_context_with_local_reset_clock() {
    let now = chrono::Local::now().timestamp();
    let five_reset = now + 2 * 3600;
    let output = render(json!({
        "context_window": context(8500, 5000, 2000),
        "rate_limits": {
            "five_hour": { "used_percentage": 23.5, "resets_at": five_reset },
            "seven_day": { "used_percentage": 91.0, "resets_at": now + 3 * 86_400 + 600 },
            "spend_limit": { "used_percentage": 10.0, "resets_at": now + 600 }
        }
    }));

    let clock = chrono::DateTime::from_timestamp(five_reset, 0)
        .unwrap()
        .with_timezone(&chrono::Local)
        .format("%H:%M")
        .to_string();
    let plain = strip_ansi(&output);
    assert!(plain.contains(&format!("5h 24% ({clock})")), "{plain}");
    assert!(
        output.contains("\x1b[90m7d\x1b[0m \x1b[38;5;203m91% (3d0h)"),
        "{output}"
    );
    assert!(plain.find("Cache(last)").unwrap() < plain.find("5h 24%").unwrap());
}

#[test]
fn api_key_sessions_without_rate_limits_hide_quota() {
    for fields in [
        json!({}),
        json!({ "rate_limits": null }),
        json!({ "rate_limits": {} }),
    ] {
        let output = render(fields);
        assert!(!output.contains("5h"), "{output}");
        assert!(!output.contains("7d"), "{output}");
    }
}
