`timescale 1ns / 1ps

module heab_gate #(
    parameter int N_ASSETS = 2000
)(
    input  logic               clk,
    input  logic               rst,
    
    // Final positions from bSB Core (Q16.16 format)
    input  logic signed [31:0] x_final [N_ASSETS-1:0],
    input  logic               bsb_done,
    input  logic               errant_momentum_ovf,
    
    output logic [31:0]        current_delta_exposure,
    output logic [31:0]        dynamic_max_notional,
    output logic               optical_kill
);

    logic [31:0] directional_exposure;
    
    always_ff @(posedge clk) begin
        if (rst) begin
            optical_kill <= 1'b1; // Default to safe/kill state
            current_delta_exposure <= 0;
            dynamic_max_notional <= 0;
        end else if (bsb_done) begin
            
            // 1. Calculate Delta Exposure
            // If x_i > 0, we allocate capital (Delta +1)
            // If x_i < 0, we bet against (Delta -1)
            logic signed [31:0] net_delta;
            logic [31:0] total_notional;
            logic [31:0] next_max_notional;
            
            net_delta = 0;
            total_notional = 0;
            
            for (int i=0; i<N_ASSETS; i++) begin
                // In Q16.16, checking sign bit
                if (x_final[i] > 0) begin
                    net_delta = net_delta + 1;
                    total_notional = total_notional + 1;
                end else if (x_final[i] < 0) begin
                    net_delta = net_delta - 1;
                    total_notional = total_notional + 1;
                end
            end
            
            // Absolute value of delta
            directional_exposure = (net_delta < 0) ? -net_delta : net_delta;
            current_delta_exposure <= directional_exposure;
            
            // 2. Determine Dynamic Margin
            if (directional_exposure < 32'd500) begin
                next_max_notional = 32'd20000; // Full $20k unlock for Arb
            end else begin
                next_max_notional = 32'd500;   // Restrict to $500 for Directional
            end
            dynamic_max_notional <= next_max_notional;
            
            // 3. Enforce Safety Constraints (HEAB logic)
            if (errant_momentum_ovf) begin
                // Q16.16 math overflowed during Euler integration, the math exploded
                optical_kill <= 1'b1; 
            end else if (total_notional > next_max_notional) begin
                // Requested trade exceeds dynamically assigned capital efficiency bounds
                optical_kill <= 1'b1;
            end else begin
                // Valid math, limits respected -> safe to transmit
                optical_kill <= 1'b0;
            end
        end
    end
endmodule
