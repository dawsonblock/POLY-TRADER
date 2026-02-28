`timescale 1ns / 1ps

module tb_bsb;

    // Use a small simulation size for verification testing
    localparam int N_ASSETS = 4;
    
    logic clk;
    logic rst;
    logic trigger;
    
    logic signed [31:0] mu [N_ASSETS-1:0];
    logic signed [31:0] sigma [N_ASSETS-1:0][N_ASSETS-1:0];
    
    wire signed [31:0] x_out [N_ASSETS-1:0];
    wire done;
    wire ovf;
    
    wire [31:0] delta_exposure;
    wire [31:0] max_notional;
    wire heab_kill;

    bsb_core #(
        .N_ASSETS(N_ASSETS),
        .MAX_ITER(10)
    ) dut_core (
        .clk(clk),
        .rst(rst),
        .trigger(trigger),
        .h_mu(mu),
        .j_sigma(sigma),
        .x_out(x_out),
        .done(done),
        .errant_momentum_ovf(ovf)
    );

    heab_gate #(
        .N_ASSETS(N_ASSETS)
    ) dut_heab (
        .clk(clk),
        .rst(rst),
        .x_final(x_out),
        .bsb_done(done),
        .errant_momentum_ovf(ovf),
        .current_delta_exposure(delta_exposure),
        .dynamic_max_notional(max_notional),
        .optical_kill(heab_kill)
    );

    always #5 clk = ~clk;

    initial begin
        clk = 0;
        rst = 1;
        trigger = 0;
        
        $display("Starting bSB and HEAB simulation...");
        
        // Initialize with safe values
        for (int i=0; i<N_ASSETS; i++) begin
            mu[i] = 32'h0001_0000; // Expected Return: +1.0 in Q16.16
            for (int j=0; j<N_ASSETS; j++) begin
                sigma[i][j] = (i==j) ? 32'h0001_0000 : 32'd0; // Identity Covariance
            end
        end
        
        #20 rst = 0;
        
        // ==========================================
        // Test 1: Normal execution
        // ==========================================
        #10 trigger = 1;
        #10 trigger = 0;
        
        wait(done);
        @(posedge clk);
        @(posedge clk);
        $display("--- Test 1: Normal Optimization ---");
        for (int i=0; i<N_ASSETS; i++) begin
            $display("Asset[%0d] Final Position: %d", i, x_out[i]);
        end
        $display("Ovf Flag: %b | HEAB Kill: %b (Expected: 0)", ovf, heab_kill);
        
        // ==========================================
        // Test 2: Forced Overflow Simulation
        // ==========================================
        // We inject a massive expected return value that will
        // exceed Q16.16 bounds during integration (int overflow wrap)
        #50;
        mu[0] = 32'h7FFF_0000; // Almost max positive value
        trigger = 1;
        #10 trigger = 0;
        
        wait(done);
        @(posedge clk);
        @(posedge clk);
        $display("--- Test 2: Forced Overflow Danger ---");
        $display("Ovf Flag: %b | HEAB Kill: %b (Expected: 1)", ovf, heab_kill);
        
        #50 $finish;
    end

endmodule
