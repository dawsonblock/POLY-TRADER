`timescale 1ns/1ps

module tb_token_bucket;

    reg clk;
    reg rst_n;
    reg attempt;
    wire violation;

    // Instantiate limited bucket for fast simulation
    boreal_token_bucket #(
        .CLKS_PER_US(10),       // Faster clock threshold for sim
        .MAX_BUCKET_SIZE(5),
        .TOKENS_PER_US(1)
    ) dut (
        .clk(clk),
        .rst_n(rst_n),
        .order_attempt(attempt),
        .rate_violation(violation)
    );

    // Clock generator
    always #5 clk = ~clk;

    initial begin
        $dumpfile("token_bucket.vcd");
        $dumpvars(0, tb_token_bucket);
        
        // Start Reset
        clk = 0;
        rst_n = 0;
        attempt = 0;
        #20;
        
        // Release Reset
        rst_n = 1;
        #10;
        
        // The bucket starts full (5 tokens). Send 5 fast orders.
        $display("[SIM] Attempting to drain full bucket...");
        repeat (5) begin
            attempt = 1;
            #10;
            attempt = 0;
            #10;
        end
        
        // 6th order should trigger a violation because of no time to refill
        $display("[SIM] Attempting 6th fast order (Should Violate)...");
        attempt = 1;
        #10;
        if (violation !== 1'b1) $display("[FAIL] Rate limit bypassed!"); else $display("[PASS] Hardware caught the burst.");
        attempt = 0;
        #10;
        
        // Wait for refill (10 clocks = 100ns)
        $display("[SIM] Waiting for token refill...");
        #150;
        
        // Try again, should pass
        attempt = 1;
        #10;
        if (violation === 1'b1) $display("[FAIL] Rate limit did not refill!"); else $display("[PASS] Token bucket refilled successfully.");
        attempt = 0;
        
        $display("[SIM] Complete.");
        $finish;
    end

endmodule
