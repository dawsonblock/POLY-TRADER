`timescale 1ns/1ps

module tb_boreal_clamp;

    reg clk;
    reg rst_n;
    reg host_valid;
    reg [15:0] host_asset;
    reg signed [63:0] host_size;
    reg signed [63:0] host_price;

    reg [63:0] max_pos [0:255];
    reg signed [63:0] max_notional;

    wire tx_enable;
    wire fault_flag;

    // Instantiate SIL-3 Clamp
    boreal_dual_clamp #(
        .NUM_ASSETS(256),
        .BIT_WIDTH(64)
    ) dut (
        .clk(clk),
        .rst_n(rst_n),
        .host_order_valid(host_valid),
        .host_asset_id(host_asset),
        .host_order_size(host_size),
        .host_order_price(host_price),
        .max_position(max_pos),
        .max_notional_per_order(max_notional),
        .tx_enable(tx_enable),
        .fault_flag(fault_flag)
    );

    // Clock generator
    always #5 clk = ~clk;

    initial begin
        $dumpfile("boreal_clamp.vcd");
        $dumpvars(0, tb_boreal_clamp);
        
        // Initialize Risk Limits
        max_notional = 64'd10000;      // max 10,000 per order
        max_pos[1] = 64'd50000;        // max 50,000 absolute inventory for asset 1

        // Reset
        clk = 0;
        rst_n = 0;
        host_valid = 0;
        host_asset = 1;
        host_size = 0;
        host_price = 0;
        
        #20 rst_n = 1;

        // Valid order within limits
        $display("[SIM] Sending 5000 size (Valid)...");
        host_valid = 1; host_size = 64'd5000;
        #10;
        if (tx_enable !== 1) $display("[FAIL] Valid order blocked!");
        host_valid = 0; #10;

        // Invalid notional size order
        $display("[SIM] Sending 15000 size (Violates Notional)...");
        host_valid = 1; host_size = 64'd15000;
        #10;
        if (tx_enable !== 0) $display("[FAIL] Over-notional order allowed!"); else $display("[PASS] Clamp caught notional breach.");
        host_valid = 0; #10;

        // Fill up to max capacity
        $display("[SIM] Filling inventory near limit...");
        max_notional = 64'd50000; // Temporarily allow bulk fill
        host_valid = 1; host_size = 64'd40000; // Total is now 45,000
        #10; host_valid = 0; #10;
        max_notional = 64'd10000; // Restore constraint

        // Try exceeding position max (45k + 6k = 51k > 50k)
        $display("[SIM] Exceeding Max Position limit...");
        host_valid = 1; host_size = 64'd6000;
        #10;
        if (tx_enable !== 0) $display("[FAIL] Over-position order allowed!"); else $display("[PASS] Clamp caught inventory breach.");

        $display("[SIM] Simulation complete.");
        $finish;
    end

endmodule
