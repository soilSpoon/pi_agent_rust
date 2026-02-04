# Providers

Pi supports multiple LLM providers. You can configure them via environment variables or `models.json`.

## Supported Providers

### Anthropic
- **Provider ID**: `anthropic`
- **Default Model**: `claude-sonnet-4-20250514`
- **Env Var**: `ANTHROPIC_API_KEY`
- **Features**: Streaming, Tools, Extended Thinking

### OpenAI
- **Provider ID**: `openai`
- **Default Model**: `gpt-4o`
- **Env Var**: `OPENAI_API_KEY`
- **Features**: Streaming, Tools

### Google Gemini
- **Provider ID**: `google`
- **Default Model**: `gemini-2.0-flash`
- **Env Var**: `GOOGLE_API_KEY` or `GEMINI_API_KEY`
- **Features**: Streaming, Tools

### Azure OpenAI
- **Provider ID**: `azure-openai`
- **Env Var**: `AZURE_OPENAI_API_KEY`
- **Features**: Streaming, Tools

## Configuration

### Environment Variables

Set these in your shell profile (e.g. `~/.bashrc` or `~/.zshrc`):

```bash
export ANTHROPIC_API_KEY="sk-ant-..."
export OPENAI_API_KEY="sk-..."
export GOOGLE_API_KEY="AIza..."
export AZURE_OPENAI_API_KEY="..."
```

### OAuth (Experimental)

Pi includes an `auth.json` mechanism for OAuth tokens, but direct environment variables are currently the recommended way to configure API keys.

## Azure OpenAI Setup

Azure OpenAI requires specifying the resource name and deployment name. Since these vary by deployment, they must be configured in `models.json` (or passed as CLI arguments if supported).

See [models.md](models.md) for details on configuring Azure OpenAI models.
