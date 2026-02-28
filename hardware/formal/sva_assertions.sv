// BOREAL FPGA: FORMAL VERIFICATION — SVA Assertions
// Add these blocks to boreal_dual_clamp.sv and token_bucket.sv
// Run with: sby -f hardware/formal/boreal_clamp.sby

// =========================================================================
// SVA for boreal_dual_clamp.sv
// Paste inside the module, after the always_ff blocks.
// =========================================================================
`ifdef FORMAL

// Property 1: Inventory never exceeds max position for any asset
property p_inventory_bound;
  @(posedge clk) disable iff (!rst_n)
  host_order_valid |->
    (pos_ram_A[host_asset_id] + host_order_size <= max_position[host_asset_id]);
endproperty
assert property (p_inventory_bound)
  else $error("INVARIANT VIOLATED: Inventory exceeds max_position!");

// Property 2: Channel disagreement ALWAYS asserts fault_flag
property p_channel_agreement;
  @(posedge clk) disable iff (!rst_n)
  (pos_ram_A[host_asset_id] != pos_ram_B[host_asset_id]) |-> fault_flag;
endproperty
assert property (p_channel_agreement)
  else $error("INVARIANT VIOLATED: Channel mismatch without fault!");

// Property 3: tx_enable is NEVER 1 when either clamp is asserted
property p_safe_tx_enable;
  @(posedge clk) disable iff (!rst_n)
  (clamp_A || clamp_B) |-> !tx_enable;
endproperty
assert property (p_safe_tx_enable)
  else $error("INVARIANT VIOLATED: tx_enable high with active clamp!");

// Cover: verify the happy path is reachable (prevents vacuous proof)
cover property (@(posedge clk) tx_enable && host_order_valid);

`endif // FORMAL


// =========================================================================
// SVA for token_bucket.sv
// Paste inside the module.
// =========================================================================
`ifdef FORMAL

// Property 4: Token count never underflows (stays >= 0)
property p_token_no_underflow;
  @(posedge clk) disable iff (!rst_n)
  tokens >= 0;
endproperty
assert property (p_token_no_underflow)
  else $error("INVARIANT VIOLATED: Token bucket underflowed!");

// Property 5: Token count never exceeds MAX_BUCKET_SIZE
property p_token_no_overflow;
  @(posedge clk) disable iff (!rst_n)
  tokens <= MAX_BUCKET_SIZE;
endproperty
assert property (p_token_no_overflow)
  else $error("INVARIANT VIOLATED: Token bucket overflowed!");

// Property 6: rate_violation only asserts when tokens == 0
property p_violation_only_when_empty;
  @(posedge clk) disable iff (!rst_n)
  rate_violation |-> (tokens == 0);
endproperty
assert property (p_violation_only_when_empty)
  else $error("INVARIANT VIOLATED: rate_violation asserted with tokens remaining!");

`endif // FORMAL
