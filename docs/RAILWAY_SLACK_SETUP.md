# Railway + Slack Deployment Guide (No NEAR AI Required)

This guide walks you through deploying IronClaw on Railway and chatting with it from Slack.

## What you'll set up

- IronClaw running on Railway
- PostgreSQL with `pgvector`
- Slack Events webhook (`/webhook/slack`)
- Slack bot replies in DMs and in channels when mentioned (`@YourBot`)
- A non-NEARAI LLM backend (for example OpenAI)

## Prerequisites

- A Railway account
- A Slack workspace where you can create apps
- An LLM API key (OpenAI, Anthropic, Ollama, or OpenAI-compatible endpoint)

---

## 1) Deploy IronClaw on Railway

### Option A: Deploy from GitHub repo (recommended)

1. In Railway, click **New Project** → **Deploy from GitHub repo**.
2. Select your IronClaw repository.
3. Add a **PostgreSQL** service in the same Railway project.
4. Railway will provide a Postgres connection URL (use it as `DATABASE_URL`).

### Option B: Deploy from a Docker image

If you prefer Docker builds, Railway can also deploy from your Dockerfile.

---

## 2) Configure Railway environment variables

Set these variables on your IronClaw service.

### Required (platform + app)

- `DATABASE_URL` = Railway Postgres URL
- `GATEWAY_ENABLED` = `true`
- `GATEWAY_HOST` = `0.0.0.0`
- `CLI_ENABLED` = `false`
- `HTTP_PORT` = `8080`
- `HTTP_HOST` = `0.0.0.0`
- `HTTP_WEBHOOK_SECRET` = long random secret string

> IronClaw supports Railway's dynamic `PORT` env var for the web gateway when `GATEWAY_PORT` is not set.

### Required (choose one LLM backend)

#### Option A: OpenAI (recommended for simplest setup)

- `LLM_BACKEND` = `openai`
- `OPENAI_API_KEY` = your API key
- `OPENAI_MODEL` = e.g. `gpt-4o-mini` (or your preferred model)

#### Option B: Anthropic

- `LLM_BACKEND` = `anthropic`
- `ANTHROPIC_API_KEY` = your API key
- `ANTHROPIC_MODEL` = e.g. `claude-sonnet-4-20250514`

#### Option C: OpenAI-compatible endpoint

- `LLM_BACKEND` = `openai_compatible`
- `LLM_BASE_URL` = endpoint base URL
- `LLM_API_KEY` = API key (if required)
- `LLM_MODEL` = model name expected by your endpoint

#### Option D: Ollama (only if reachable from Railway runtime)

- `LLM_BACKEND` = `ollama`
- `OLLAMA_BASE_URL` = Ollama URL
- `OLLAMA_MODEL` = model name

### Optional (NEAR AI)

You can still use NEAR AI, but it is **not required** for Railway + Slack.

### Slack-related secrets

Slack channel credentials are stored in IronClaw's encrypted secrets store. The easiest way is to run onboarding once and save these secret names:

- `slack_bot_token`
- `slack_signing_secret`

If you are automating setup, ensure the same secret names are present in the secrets store for the runtime user.

---

## 3) Install and enable the Slack channel

On the machine/environment where you prepare runtime data:

```bash
ironclaw onboard --channels-only
```

When prompted:

- enable **Slack** channel
- provide Slack bot token (`xoxb-...`)
- provide Slack signing secret

This installs channel files under `~/.ironclaw/channels/` and registers Slack credentials in the secrets store.

---


## Channel + DM behavior (important)

IronClaw's Slack channel currently processes:

- **DMs**: direct messages to the bot (`message.im`)
- **Channels**: only messages that **mention** the bot (`app_mention`)

So for channel conversations:

1. Invite the bot into the channel (`/invite @YourBot`)
2. Mention it in each message you want processed (`@YourBot ...`)

If you send a channel message without mentioning the bot, IronClaw will ignore it by design.

---

## 4) Create and configure your Slack app

1. Go to <https://api.slack.com/apps> and create an app.
2. Under **OAuth & Permissions**, add bot scopes:
   - `app_mentions:read`
   - `channels:history` (public channels)
   - `groups:history` (private channels, if needed)
   - `chat:write`
   - `im:history` (DMs)
3. Install the app to your workspace and copy the **Bot User OAuth Token** (`xoxb-...`).
4. Under **Event Subscriptions**:
   - Enable events
   - Set **Request URL** to:
     - `https://<your-railway-domain>/webhook/slack`
   - Subscribe to bot events:
     - `app_mention`
     - `message.im`
5. Under **Basic Information**, copy the **Signing Secret**.

---

## 5) Route webhook traffic correctly

Slack events are served by IronClaw's HTTP webhook channel endpoint:

- `POST /webhook/slack`

Make sure your Railway service allows inbound HTTPS and your Slack app points to the public Railway URL.

If Slack URL verification fails:

- confirm your service is healthy and reachable
- confirm path is exactly `/webhook/slack`
- confirm `HTTP_WEBHOOK_SECRET` and signing secret setup are correct

---

## 6) Validate end-to-end

1. DM your Slack bot with a message.
2. Mention the bot in a channel: `@YourBot hello`.
3. Check Railway logs for webhook delivery and response messages.

Expected behavior:

- DMs should be processed.
- Mentions should be processed.
- Bot messages should not trigger loops.

---

## Troubleshooting

### Railway service is up, but Slack gets timeout

- Slack requires fast webhook ACKs (~3s).
- Ensure app startup succeeded and DB connection works.
- Check logs for startup/config errors.

### Startup fails with missing LLM credentials

- Verify your selected `LLM_BACKEND` matches the env vars you set.
- For OpenAI: ensure `OPENAI_API_KEY` is set.
- For Anthropic: ensure `ANTHROPIC_API_KEY` is set.
- For OpenAI-compatible: ensure `LLM_BASE_URL` is set.

### Slack sends events but bot doesn't respond

- Confirm `slack_bot_token` exists in secrets store.
- Verify bot has `chat:write` scope.
- Reinstall app after changing scopes.
- For channel messages: ensure the bot was invited and you used an `@mention`.
- For private channels: ensure `groups:history` scope is added and app reinstalled.

### Can't bind the right port on Railway

- Leave `GATEWAY_PORT` unset and rely on `PORT`, or set `GATEWAY_PORT` explicitly.
- Keep `GATEWAY_HOST=0.0.0.0`.

### PostgreSQL errors

- Ensure `pgvector` extension is enabled.
- Verify `DATABASE_URL` points to the Railway Postgres instance.
