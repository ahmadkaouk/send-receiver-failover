# Sender - Receiver - Failover
An experiment to implement an automated Atomic Failover System for a Sender-Receiver App. The system is designed to maximize application uptime. The app is fully written in rust, with an attempt to use as mush as possible the standard libray tools while keeping the solution simple and writing a minimal code.

## Logic
The App run in 3 modes:
* **Receiver:** A Receiver App receives the payload from the Sender App and print each one of them.
* **Sender:** The primary Sender App increments a counter every second (infinitely or until interruption), and sends each count number to the Receiver App.
* **Failover:** A Backup App that automatically switch to a Sender when the primary Sender App fails(Stops or get interrupted)

You will maybe need root priviliges to run the Apps.

## Atomic Failover System
The automated Failover feature is supported through a redundant Sender App and a Heartbeat System. The primary Sender App sends periodically heartbeat signals [Status<Success|Fail> : Count] to the redundant App (Failover). If the Failover System receives a signal with a fail status or don't receive a signal whithin 2 seconds, it will start sending payloads to the Receiver App with the count that was left of and with "slave" as node_id.

## Running
Run in separate terminals (in order):
```rust
cargo run -- Receiver
cargo run -- Sender
cargo run -- Failover
```

## Testing
Two scenarios can be run to test the Failover System:
1. The Sender App sends a fail signal to Failover App
2. The Failover App does not receive a signal from the Sender App in 2 seconds.

To test the first scenarios, run the 3 Apss as descirbed in the section above, and then try to stop the Sender App by using Ctrl-C. In this case, the Failover App should start sending payloads immediately to Receiver. (Node_id in payloads printed by the Receiver should be "slave").

The second scenario can be tested by killing the Sender App manually:
```
$ ps aux | grep -i send
sesame     44208  0.3  0.0   5144  1028 pts/4    S+   09:56   0:00 target/debug/send_receive_failover Receiver
sesame     44214  0.5  0.0  72744  1024 pts/0    Sl+  09:56   0:00 target/debug/send_receive_failover Sender
sesame     44226  0.6  0.0  72744  1024 pts/2    Sl+  09:56   0:00 target/debug/send_receive_failover Failover
sesame     44306  0.0  0.0   8108  4060 pts/5    D+   09:56   0:00 rg -i send
$ sudo kill 44214
```
After two seconds, the Failover App should start sending payloads to Receiver (Node_id in payloads printed by the Receiver should be "slave").