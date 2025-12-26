use anyhow::Result;
use crate::process_monitor::ProcessInfo;

#[derive(Debug, Clone)]
pub struct ReactAbuseDetection {
    pub pid: i32,
    pub binary_path: String,
    pub confidence: f32,
    pub reasons: Vec<String>,
}

pub struct ReactDetector {
    // Heuristic-based detection for React Flight protocol abuse
}

impl ReactDetector {
    pub fn new() -> Self {
        Self {}
    }

    pub fn detect(&self, process: &ProcessInfo, cpu_percent: f32) -> Option<ReactAbuseDetection> {
        let mut confidence = 0.0;
        let mut reasons = Vec::new();

        // Check if this is a Node.js process
        if !process.binary_path.contains("node") && !process.command_line.contains("node") {
            return None;
        }

        // Heuristic 1: Node process handling serialized payloads
        // Look for common React server patterns
        if process.command_line.contains("react") 
            || process.command_line.contains("next")
            || process.command_line.contains("remix") {
            
            // Heuristic 2: High CPU during idle time (suspicious for mining)
            if cpu_percent > 15.0 {
                confidence += 0.3;
                reasons.push("High CPU in React server process".to_string());
            }

            // Heuristic 3: Long-running deserialization loops
            // This is harder to detect without deeper inspection, but we can
            // look for processes that have been running a long time with high CPU
            if cpu_percent > 20.0 {
                confidence += 0.2;
                reasons.push("Sustained high CPU in React handler".to_string());
            }
        }

        // Heuristic 4: Child processes spawned from React handlers
        // This would require tracking process trees, which we do in process_monitor
        // For now, we check command line for suspicious patterns

        // Heuristic 5: Check for crypto-related modules in command line
        if process.command_line.contains("crypto") 
            || process.command_line.contains("miner")
            || process.command_line.contains("hash") {
            confidence += 0.4;
            reasons.push("Crypto-related code in React process".to_string());
        }

        // Heuristic 6: Check for obfuscated or minified code execution
        if process.command_line.contains("eval") 
            || process.command_line.contains("Function(") {
            confidence += 0.3;
            reasons.push("Dynamic code execution detected".to_string());
        }

        if confidence > 0.5 {
            Some(ReactAbuseDetection {
                pid: process.pid,
                binary_path: process.binary_path.clone(),
                confidence,
                reasons,
            })
        } else {
            None
        }
    }
}

