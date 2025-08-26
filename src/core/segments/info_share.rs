use super::Segment;
use crate::config::InputData;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, Deserialize)]
struct ClaudeCodeSettings {
    info_share_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct InfoShareResponse {
    account_id: String,
    date: String,
    timezone: String,
    peers: Vec<Peer>,
}

#[derive(Debug, Deserialize)]
struct Peer {
    user_id: String,
    display_name: String,
    spent_usd_today: String,
}

pub struct InfoShareSegment {
    enabled: bool,
    info_share_url: Option<String>,
    jwt_token: Option<String>,
}

impl InfoShareSegment {
    pub fn new(enabled: bool) -> Self {
        let (info_share_url, jwt_token) = Self::load_config();
        Self {
            enabled,
            info_share_url,
            jwt_token,
        }
    }

    fn load_config() -> (Option<String>, Option<String>) {
        // 1. 从 Claude Code settings.json 读取 info_share_url
        let info_share_url = if let Some(config_dir) = Self::get_claude_config_dir() {
            let settings_path = config_dir.join("settings.json");
            if let Ok(content) = fs::read_to_string(&settings_path) {
                if let Ok(settings) = serde_json::from_str::<ClaudeCodeSettings>(&content) {
                    settings.info_share_url
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        // 2. 从环境变量读取 PARCKY_JWT_TOKEN
        let jwt_token = std::env::var("PARCKY_JWT_TOKEN").ok();

        (info_share_url, jwt_token)
    }

    fn get_claude_config_dir() -> Option<PathBuf> {
        // Claude Code 配置目录是 ~/.claude
        dirs::home_dir().map(|home| home.join(".claude"))
    }

    fn fetch_info(&self) -> Option<f64> {
        let url = self.info_share_url.as_ref()?;
        let token = self.jwt_token.as_ref()?;

        let client = reqwest::blocking::Client::new();
        let response = client
            .get(url)
            .header("Authorization", format!("Bearer {}", token))
            .header("accept", "*/*")
            .header("content-type", "application/json")
            .timeout(Duration::from_secs(5))
            .send()
            .ok()?;

        if response.status().is_success() {
            // 解析API响应
            if let Ok(info_data) = response.json::<InfoShareResponse>() {
                // 累加所有 peers 的 spent_usd_today
                let total_peers_spent: f64 = info_data.peers
                    .iter()
                    .map(|peer| peer.spent_usd_today.parse::<f64>().unwrap_or(0.0))
                    .sum();
                
                return Some(total_peers_spent);
            }
        }

        None
    }
}

impl Segment for InfoShareSegment {
    fn render(&self, input: &InputData) -> String {
        if !self.enabled || self.info_share_url.is_none() || self.jwt_token.is_none() {
            return String::new();
        }

        // 获取团队成员（peers）的总消费
        let peers_total = self.fetch_info().unwrap_or(0.0);
        
        // 获取当前用户的消费
        // 创建一个 QuotaSegment 实例来获取用户消费
        let quota_segment = super::QuotaSegment::new(true);
        let user_quota_str = quota_segment.render(input);
        
        // 解析用户的 Today 值 (格式: "Today: $X.XX" 或 "Today: $X.XX | Month: $Y.YY | ...")
        let user_today = if user_quota_str.starts_with("Today: $") {
            let parts: Vec<&str> = user_quota_str.split('|').collect();
            if let Some(today_part) = parts.first() {
                today_part
                    .trim_start_matches("Today: $")
                    .trim()
                    .parse::<f64>()
                    .unwrap_or(0.0)
            } else {
                0.0
            }
        } else {
            0.0
        };
        
        // 计算团队总消费 = peers总消费 + 用户自己的消费
        let total_team_spent = peers_total + user_today;
        
        format!("Team Total: ${:.2}", total_team_spent)
    }

    fn enabled(&self) -> bool {
        self.enabled && self.info_share_url.is_some() && self.jwt_token.is_some()
    }
}