# WideScope Conventions

This directory contains community-maintained attribute mapping files that tell WideScope how to extract LLM-specific metadata from spans.

## Format

Each file is a JSON object with the following structure:

```json
{
  "name": "Convention name",
  "version": "x.y.z",
  "detect": {
    "attribute_prefix": "gen_ai.",
    "any_key_present": ["gen_ai.system"]
  },
  "mappings": {
    "model_name": { "attribute": "gen_ai.request.model" },
    "input_tokens": { "attribute": "gen_ai.usage.input_tokens", "type": "int" }
  }
}
```

## Priority

Conventions are applied in order: `opentelemetry.json` → `openinference.json` → `langchain.json`. First match wins.

## Contributing

To add a new framework mapping, open a PR with a new JSON file following the schema above.
