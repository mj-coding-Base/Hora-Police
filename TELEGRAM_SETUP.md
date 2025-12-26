# 📱 Telegram Setup Guide

This guide will help you set up Telegram reporting for Sentinel Anti-Malware Daemon.

## Step 1: Create a Telegram Bot

1. Open Telegram and search for **@BotFather**
2. Start a conversation and send `/newbot`
3. Follow the prompts:
   - Choose a name for your bot (e.g., "My Sentinel Bot")
   - Choose a username (must end in `bot`, e.g., "my_sentinel_bot")
4. **Copy the bot token** - it will look like:
   ```
   123456789:ABCdefGHIjklMNOpqrsTUVwxyz
   ```
   ⚠️ **Keep this token secret!**

## Step 2: Get Your Chat ID

You have two options:

### Option A: Use Username (Simpler)
If your Telegram username is `mjpavithra`, you can use:
```
chat_id = "@mjpavithra"
```

### Option B: Get Numeric Chat ID (More Reliable)
1. Search for **@userinfobot** on Telegram
2. Start a conversation - it will reply with your user ID
3. Copy the numeric ID (e.g., `123456789`)

## Step 3: Test Your Bot

1. Send a message to your bot (search for it by username)
2. The bot should receive your message (even if it doesn't reply yet)

## Step 4: Configure Sentinel

Edit `/etc/sentinel/config.toml`:

```toml
[telegram]
bot_token = "123456789:ABCdefGHIjklMNOpqrsTUVwxyz"  # Your bot token
chat_id = "@mjpavithra"  # Your username or numeric ID
daily_report_time = "09:00"  # Time for daily report (24-hour format)
```

## Step 5: Enable Real-Time Alerts (Optional)

If you want immediate notifications when threats are detected:

```toml
real_time_alerts = true
```

⚠️ **Warning**: This can generate many messages if your system is under attack. Start with `false` and enable only if needed.

## Step 6: Restart Sentinel

```bash
sudo systemctl restart sentinel
```

## Step 7: Test Telegram Integration

You can manually trigger a test by checking the logs:

```bash
sudo journalctl -u sentinel -f
```

Wait for the next daily report time, or check if real-time alerts are working.

## Troubleshooting

### Bot Not Receiving Messages

1. **Check bot token**: Make sure it's correct and hasn't been revoked
2. **Check chat ID**: Verify your username or numeric ID is correct
3. **Start conversation**: Make sure you've sent at least one message to your bot

### No Daily Reports

1. **Check time format**: Must be `HH:MM` (24-hour format)
2. **Check timezone**: Reports use system timezone
3. **Check logs**: `sudo journalctl -u sentinel -f` for errors

### API Errors

If you see Telegram API errors in logs:
- Verify bot token is valid
- Check internet connectivity
- Ensure Telegram API is accessible (not blocked by firewall)

## Example Daily Report

You'll receive reports like this:

```
🛡️ Sentinel Daily Report

Summary:
• Processes Killed: 3
• Suspicious Processes: 5
• npm Infections: 1

Recent Actions:
• PID 12345 (/tmp/miner) - 85% confidence
  Reason: CPU abuse: 45% for 600 seconds
• PID 12346 (/home/user/node_modules/evil-package) - 90% confidence
  Reason: CPU abuse (30% for 400s) + npm infection: evil-package
```

## Security Notes

- **Bot Token**: Treat it like a password - never commit to version control
- **Chat ID**: Can be your username (public) or numeric ID (private)
- **Rate Limits**: Telegram has rate limits, but daily reports won't hit them

## Advanced: Multiple Recipients

To send to multiple chats, you'll need to modify the code or run multiple bot instances with different chat IDs.

---

**Need Help?** Check the main README.md or open an issue.

