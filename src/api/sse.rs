#[derive(Debug)]
pub struct SSEvent {
    pub id: Option<String>,
    pub event: Option<String>,
    pub data: Option<String>,
    pub retry: Option<u32>,
}

pub fn parse_sse_chunk(data: &str, buffer: &mut String) -> Vec<SSEvent> {
    buffer.push_str(data);

    let mut events = Vec::new();

    while let Some(index) = buffer.find("\n\n") {
        let event_data = buffer.drain(..index).collect::<String>();
        buffer.drain(0..2);

        events.extend(parse_sse(&event_data));
    }

    events
}

fn parse_sse(data: &str) -> Vec<SSEvent> {
    let mut events = Vec::new();

    for line in data.split("\n\n") {
        let mut event = SSEvent {
            id: None,
            event: None,
            data: None,
            retry: None,
        };

        for entry in line.split('\n') {
            let (field, value) = {
                let parts: Vec<&str> = entry.splitn(2, ':').collect();
                (parts[0].trim(), parts.get(1).unwrap_or(&"").trim())
            };

            match field {
                "id" => event.id = Some(value.to_string()),
                "event" => event.event = Some(value.to_string()),
                "data" => event.data = Some(value.to_string()),
                "retry" => event.retry = value.parse().ok(),
                _ => {}
            }
        }

        events.push(event);
    }

    events
}
