-- Initialize the code length settings
INSERT INTO settings (key, value, updated_at) 
VALUES 
  ('min_random_code_length', '6', 0),
  ('min_custom_code_length', '3', 0),
  ('system_min_code_length', '1', 0);
  