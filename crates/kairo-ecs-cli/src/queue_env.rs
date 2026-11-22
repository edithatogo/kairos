use std::io::{self, BufRead, Read, Write};

use kairo_ecs_ml::{QueueEnvironment, QueueObservation};
use serde::{Deserialize, Serialize};

const MAX_COMMAND_BYTES: u64 = 512;

#[derive(Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
enum Command {
    Reset { seed: u64 },
    Step { actions: Vec<i64> },
}

#[derive(Serialize)]
#[serde(untagged)]
enum Reply {
    Ready {
        model: &'static str,
        max_waiting: u32,
        arrival_ticks: u32,
    },
    Reset {
        observation: QueueObservation,
    },
    Step {
        observation: QueueObservation,
        rewards: Vec<f32>,
        terminated: bool,
        truncated: bool,
    },
    Error {
        error: String,
    },
}

pub fn run(agents: usize, max_ticks: u32) -> Result<(), String> {
    let env = QueueEnvironment::new(agents, max_ticks, 0).map_err(|e| e.to_string())?;
    serve(env, io::stdin().lock(), io::stdout().lock()).map_err(|e| e.to_string())
}

fn serve(
    mut env: QueueEnvironment,
    mut input: impl BufRead,
    mut output: impl Write,
) -> io::Result<()> {
    write_reply(
        &mut output,
        Reply::Ready {
            model: QueueEnvironment::VERSION,
            max_waiting: QueueEnvironment::MAX_WAITING,
            arrival_ticks: QueueEnvironment::ARRIVAL_TICKS,
        },
    )?;
    let mut line = Vec::new();
    loop {
        line.clear();
        if (&mut input)
            .take(MAX_COMMAND_BYTES)
            .read_until(b'\n', &mut line)?
            == 0
        {
            return Ok(());
        }
        if !line.ends_with(b"\n") {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "queue command exceeds frame limit or has no newline",
            ));
        }
        let reply = match serde_json::from_slice(&line) {
            Ok(Command::Reset { seed }) => Reply::Reset {
                observation: env.reset(seed),
            },
            Ok(Command::Step { actions }) => match env.step(&actions) {
                Ok(step) => Reply::Step {
                    observation: step.observation,
                    rewards: step.rewards,
                    terminated: step.terminated,
                    truncated: step.truncated,
                },
                Err(error) => Reply::Error {
                    error: error.to_string(),
                },
            },
            Err(error) => Reply::Error {
                error: error.to_string(),
            },
        };
        write_reply(&mut output, reply)?;
    }
}

fn write_reply(output: &mut impl Write, reply: Reply) -> io::Result<()> {
    serde_json::to_writer(&mut *output, &reply)?;
    output.write_all(b"\n")?;
    output.flush()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    #[test]
    fn invalid_wire_commands_leave_native_state_and_rng_intact() {
        let mut output = Vec::new();
        let script = b"{\"op\":\"reset\",\"seed\":1}\n{\"op\":\"step\",\"actions\":[2]}\n{\"op\":\"step\",\"actions\":[0],\"extra\":true}\n{\"op\":\"step\",\"actions\":[1]}\n";
        let env = QueueEnvironment::new(1, 8, 0).unwrap();
        serve(env, &script[..], &mut output).unwrap();
        let replies: Vec<Value> = output
            .lines()
            .map(|line| serde_json::from_str(&line.unwrap()).unwrap())
            .collect();
        assert_eq!(replies[0]["model"], QueueEnvironment::VERSION);
        assert_eq!(replies[1]["observation"], serde_json::json!([0, 2, 8]));
        assert!(replies[2]["error"].is_string());
        assert!(replies[3]["error"].is_string());

        let expected = QueueEnvironment::new(1, 8, 1).unwrap().step(&[1]).unwrap();
        assert_eq!(
            replies[4]["observation"],
            serde_json::json!(expected.observation)
        );
        assert_eq!(replies[4]["rewards"], serde_json::json!(expected.rewards));
    }

    #[test]
    fn oversized_and_unterminated_frames_fail_closed() {
        let env = || QueueEnvironment::new(1, 8, 0).unwrap();
        let oversized = vec![b' '; MAX_COMMAND_BYTES as usize + 1];
        for input in [&oversized[..], b"{}"] {
            let error = serve(env(), input, Vec::new()).unwrap_err();
            assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        }
    }
}
