//! Notification module for Slack, Discord, and Telegram integrations.
//!
//! Configure via environment variables:
//! - `LAMBDA_NOTIFY_SLACK_WEBHOOK` - Slack incoming webhook URL
//! - `LAMBDA_NOTIFY_DISCORD_WEBHOOK` - Discord webhook URL
//! - `LAMBDA_NOTIFY_TELEGRAM_BOT_TOKEN` - Telegram bot token
//! - `LAMBDA_NOTIFY_TELEGRAM_CHAT_ID` - Telegram chat ID

use anyhow::Result;
use reqwest::Client;
use serde_json::json;
use std::time::Duration;

/// Message payload for instance ready notifications
#[derive(Debug, Clone)]
pub struct InstanceReadyMessage {
    pub instance_id: String,
    pub instance_name: Option<String>,
    pub ip: String,
    pub gpu_type: String,
    pub region: String,
}

impl InstanceReadyMessage {
    pub fn ssh_command(&self) -> String {
        format!("ssh ubuntu@{}", self.ip)
    }

    pub fn display_name(&self) -> &str {
        self.instance_name.as_deref().unwrap_or(&self.instance_id)
    }
}

/// Slack webhook configuration
#[derive(Debug, Clone)]
pub struct SlackConfig {
    pub webhook_url: String,
}

/// Discord webhook configuration
#[derive(Debug, Clone)]
pub struct DiscordConfig {
    pub webhook_url: String,
}

/// Telegram bot configuration
#[derive(Debug, Clone)]
pub struct TelegramConfig {
    pub bot_token: String,
    pub chat_id: String,
}

/// Combined notification configuration
#[derive(Debug, Clone, Default)]
pub struct NotifyConfig {
    pub slack: Option<SlackConfig>,
    pub discord: Option<DiscordConfig>,
    pub telegram: Option<TelegramConfig>,
}

impl NotifyConfig {
    /// Load notification configuration from environment variables
    pub fn from_env() -> Option<Self> {
        let slack = std::env::var("LAMBDA_NOTIFY_SLACK_WEBHOOK")
            .ok()
            .filter(|s| !s.is_empty())
            .map(|webhook_url| SlackConfig { webhook_url });

        let discord = std::env::var("LAMBDA_NOTIFY_DISCORD_WEBHOOK")
            .ok()
            .filter(|s| !s.is_empty())
            .map(|webhook_url| DiscordConfig { webhook_url });

        let telegram = match (
            std::env::var("LAMBDA_NOTIFY_TELEGRAM_BOT_TOKEN"),
            std::env::var("LAMBDA_NOTIFY_TELEGRAM_CHAT_ID"),
        ) {
            (Ok(bot_token), Ok(chat_id)) if !bot_token.is_empty() && !chat_id.is_empty() => {
                Some(TelegramConfig { bot_token, chat_id })
            }
            _ => None,
        };

        if slack.is_some() || discord.is_some() || telegram.is_some() {
            Some(Self {
                slack,
                discord,
                telegram,
            })
        } else {
            None
        }
    }

    /// Check if any notification channel is configured
    pub fn is_configured(&self) -> bool {
        self.slack.is_some() || self.discord.is_some() || self.telegram.is_some()
    }

    /// Get a list of configured notification channels (for display)
    pub fn configured_channels(&self) -> Vec<&'static str> {
        let mut channels = Vec::new();
        if self.slack.is_some() {
            channels.push("Slack");
        }
        if self.discord.is_some() {
            channels.push("Discord");
        }
        if self.telegram.is_some() {
            channels.push("Telegram");
        }
        channels
    }
}

/// Notifier for sending messages to configured channels
pub struct Notifier {
    client: Client,
    config: NotifyConfig,
}

impl Notifier {
    /// Create a new notifier with the given configuration
    pub fn new(config: NotifyConfig) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .expect("Failed to create HTTP client");

        Self { client, config }
    }

    /// Create a notifier from environment variables, if configured
    pub fn from_env() -> Option<Self> {
        NotifyConfig::from_env().map(Self::new)
    }

    /// Send notification to all configured channels
    ///
    /// Returns a list of (channel_name, result) for each attempt
    pub async fn send_all(&self, msg: &InstanceReadyMessage) -> Vec<(&'static str, Result<()>)> {
        let mut results = Vec::new();

        if let Some(ref slack) = self.config.slack {
            let result = self.send_slack(slack, msg).await;
            results.push(("Slack", result));
        }

        if let Some(ref discord) = self.config.discord {
            let result = self.send_discord(discord, msg).await;
            results.push(("Discord", result));
        }

        if let Some(ref telegram) = self.config.telegram {
            let result = self.send_telegram(telegram, msg).await;
            results.push(("Telegram", result));
        }

        results
    }

    /// Send notification to Slack
    async fn send_slack(&self, config: &SlackConfig, msg: &InstanceReadyMessage) -> Result<()> {
        let payload = json!({
            "blocks": [
                {
                    "type": "header",
                    "text": {
                        "type": "plain_text",
                        "text": "GPU Instance Ready!",
                        "emoji": true
                    }
                },
                {
                    "type": "section",
                    "fields": [
                        {
                            "type": "mrkdwn",
                            "text": format!("*Name:*\n{}", msg.display_name())
                        },
                        {
                            "type": "mrkdwn",
                            "text": format!("*GPU:*\n{}", msg.gpu_type)
                        },
                        {
                            "type": "mrkdwn",
                            "text": format!("*Region:*\n{}", msg.region)
                        },
                        {
                            "type": "mrkdwn",
                            "text": format!("*IP:*\n{}", msg.ip)
                        }
                    ]
                },
                {
                    "type": "section",
                    "text": {
                        "type": "mrkdwn",
                        "text": format!("*SSH Command:*\n```{}```", msg.ssh_command())
                    }
                }
            ]
        });

        let response = self
            .client
            .post(&config.webhook_url)
            .json(&payload)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Slack webhook failed ({}): {}", status, body);
        }

        Ok(())
    }

    /// Send notification to Discord
    async fn send_discord(&self, config: &DiscordConfig, msg: &InstanceReadyMessage) -> Result<()> {
        let payload = json!({
            "embeds": [{
                "title": "GPU Instance Ready!",
                "color": 5763719,  // Green color
                "fields": [
                    {
                        "name": "Name",
                        "value": msg.display_name(),
                        "inline": true
                    },
                    {
                        "name": "GPU",
                        "value": msg.gpu_type,
                        "inline": true
                    },
                    {
                        "name": "Region",
                        "value": msg.region,
                        "inline": true
                    },
                    {
                        "name": "IP Address",
                        "value": msg.ip,
                        "inline": true
                    },
                    {
                        "name": "SSH Command",
                        "value": format!("```{}```", msg.ssh_command()),
                        "inline": false
                    }
                ]
            }]
        });

        let response = self
            .client
            .post(&config.webhook_url)
            .json(&payload)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Discord webhook failed ({}): {}", status, body);
        }

        Ok(())
    }

    /// Send notification to Telegram
    async fn send_telegram(
        &self,
        config: &TelegramConfig,
        msg: &InstanceReadyMessage,
    ) -> Result<()> {
        let url = format!(
            "https://api.telegram.org/bot{}/sendMessage",
            config.bot_token
        );

        let text = format!(
            "*GPU Instance Ready\\!*\n\n\
             *Name:* `{}`\n\
             *GPU:* {}\n\
             *Region:* {}\n\
             *IP:* `{}`\n\n\
             *SSH Command:*\n```\n{}\n```",
            escape_telegram_markdown(msg.display_name()),
            escape_telegram_markdown(&msg.gpu_type),
            escape_telegram_markdown(&msg.region),
            msg.ip,
            msg.ssh_command()
        );

        let payload = json!({
            "chat_id": config.chat_id,
            "parse_mode": "MarkdownV2",
            "text": text
        });

        let response = self.client.post(&url).json(&payload).send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Telegram API failed ({}): {}", status, body);
        }

        Ok(())
    }
}

/// Escape special characters for Telegram MarkdownV2
fn escape_telegram_markdown(text: &str) -> String {
    let special_chars = [
        '_', '*', '[', ']', '(', ')', '~', '`', '>', '#', '+', '-', '=', '|', '{', '}', '.', '!',
    ];
    let mut result = String::with_capacity(text.len() * 2);
    for c in text.chars() {
        if special_chars.contains(&c) {
            result.push('\\');
        }
        result.push(c);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_telegram_markdown() {
        assert_eq!(escape_telegram_markdown("hello"), "hello");
        assert_eq!(escape_telegram_markdown("hello_world"), "hello\\_world");
        assert_eq!(escape_telegram_markdown("gpu_1x_a100"), "gpu\\_1x\\_a100");
    }

    #[test]
    fn test_instance_ready_message() {
        let msg = InstanceReadyMessage {
            instance_id: "abc123".to_string(),
            instance_name: Some("my-gpu".to_string()),
            ip: "1.2.3.4".to_string(),
            gpu_type: "gpu_1x_a100".to_string(),
            region: "us-east-1".to_string(),
        };

        assert_eq!(msg.ssh_command(), "ssh ubuntu@1.2.3.4");
        assert_eq!(msg.display_name(), "my-gpu");

        let msg_no_name = InstanceReadyMessage {
            instance_id: "abc123".to_string(),
            instance_name: None,
            ip: "1.2.3.4".to_string(),
            gpu_type: "gpu_1x_a100".to_string(),
            region: "us-east-1".to_string(),
        };

        assert_eq!(msg_no_name.display_name(), "abc123");
    }

    #[test]
    fn test_notify_config_is_configured() {
        let empty = NotifyConfig::default();
        assert!(!empty.is_configured());

        let with_slack = NotifyConfig {
            slack: Some(SlackConfig {
                webhook_url: "https://hooks.slack.com/test".to_string(),
            }),
            ..Default::default()
        };
        assert!(with_slack.is_configured());
        assert_eq!(with_slack.configured_channels(), vec!["Slack"]);
    }
}
