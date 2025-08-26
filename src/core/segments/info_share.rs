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

    fn fetch_team_total(&self) -> Option<f64> {
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
    
    fn format_team_total(&self, peers_total: f64, user_total: f64) -> String {
        // 计算团队总消费并格式化
        let team_total = peers_total + user_total;
        format!("Team: ${:.2}", team_total)
    }
}

impl Segment for InfoShareSegment {
    fn render(&self, input: &InputData) -> String {
        if !self.enabled || self.info_share_url.is_none() || self.jwt_token.is_none() {
            return String::new();
        }

        // 获取团队成员（peers）的总消费
        if let Some(peers_total) = self.fetch_team_total() {
            // 获取当前用户的消费
            let quota_segment = super::QuotaSegment::new(true);
            let user_quota_str = quota_segment.render(input);
            
            // 解析用户的 Today 值
            let user_today = if user_quota_str.starts_with("Today: $") {
                user_quota_str
                    .trim_start_matches("Today: $")
                    .trim()
                    .parse::<f64>()
                    .unwrap_or(0.0)
            } else {
                0.0
            };
            
            // 格式化团队总消费
            self.format_team_total(peers_total, user_today)
        } else {
            // 如果无法获取团队数据，显示 N/A
            "◔ Team: N/A".to_string()
        }
    }

    fn enabled(&self) -> bool {
        self.enabled && self.info_share_url.is_some() && self.jwt_token.is_some()
    }
}