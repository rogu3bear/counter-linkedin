CREATE TABLE IF NOT EXISTS generation_events (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  client_hash TEXT NOT NULL,
  host TEXT NOT NULL,
  route TEXT NOT NULL,
  mode TEXT,
  intensity INTEGER,
  regenerate INTEGER NOT NULL DEFAULT 0,
  model_name TEXT,
  input_text TEXT NOT NULL,
  input_chars INTEGER NOT NULL,
  output_text TEXT,
  output_chars INTEGER,
  prompt_tokens INTEGER,
  completion_tokens INTEGER,
  total_tokens INTEGER,
  estimated_input_cost_usd REAL,
  estimated_output_cost_usd REAL,
  estimated_total_cost_usd REAL,
  latency_ms INTEGER,
  status TEXT NOT NULL,
  error_code TEXT,
  error_message TEXT,
  warnings_json TEXT
);

CREATE INDEX IF NOT EXISTS idx_generation_events_created_at
  ON generation_events (created_at DESC);

CREATE INDEX IF NOT EXISTS idx_generation_events_status_created_at
  ON generation_events (status, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_generation_events_mode_created_at
  ON generation_events (mode, created_at DESC);
