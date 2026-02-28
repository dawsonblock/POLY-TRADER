/*
 * BOREAL FPGA: TOKEN BUCKET RATE LIMITER
 * Purpose: Enforces maximum orders per microsecond to prevent runaway software 
 * loops from draining exchange connectivity limits.
 */

module boreal_token_bucket #(
    parameter CLKS_PER_US = 500,  // Assuming 500MHz clock
    parameter MAX_BUCKET_SIZE = 5, 
    parameter TOKENS_PER_US = 1
)(
    input  wire clk,
    input  wire rst_n,

    input  wire order_attempt,    // Software wants to send an order
    output reg  rate_violation    // 1 if empty. Packet must be dropped!
);

    reg [31:0] clk_counter;
    reg [7:0] tokens;

    always_ff @(posedge clk) begin
        if (!rst_n) begin
            clk_counter <= 0;
            tokens <= MAX_BUCKET_SIZE;
            rate_violation <= 0;
        end else begin
            // 1. Refill Logic
            if (clk_counter >= CLKS_PER_US - 1) begin
                clk_counter <= 0;
                if (tokens + TOKENS_PER_US <= MAX_BUCKET_SIZE) begin
                    tokens <= tokens + TOKENS_PER_US;
                end else begin
                    tokens <= MAX_BUCKET_SIZE;
                end
            end else begin
                clk_counter <= clk_counter + 1;
            end

            // 2. Consume Logic
            if (order_attempt) begin
                if (tokens > 0) begin
                    tokens <= tokens - 1;
                    rate_violation <= 0; // Safe to pass
                end else begin
                    rate_violation <= 1; // Burn the packet!
                end
            end else begin
                rate_violation <= 0;
            end
        end
    end

endmodule
