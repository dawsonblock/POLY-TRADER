`timescale 1ns / 1ps

module bsb_core #(
    parameter int N_ASSETS = 2000,
    parameter int MAX_ITER = 100
)(
    input  logic               clk,
    input  logic               rst,
    input  logic               trigger,
    
    // Q16.16 Expected Return (Zeeman Field)
    input  logic signed [31:0] h_mu [N_ASSETS-1:0],
    
    // Q16.16 Covariance (Interaction Matrix J)
    // Note: In a fully pipelined synthesis build, this would map to an AXI stream 
    // or block RAM interface. Modeled as a flat array for architectural behavioral definition.
    input  logic signed [31:0] j_sigma [N_ASSETS-1:0][N_ASSETS-1:0],
    
    // Results: Q16.16 Position
    output logic signed [31:0] x_out [N_ASSETS-1:0],
    output logic               done,
    output logic               errant_momentum_ovf
);

    // Q16.16 Constants
    localparam signed [31:0] DT = 32'h0000_1000; // Delta t (approx 0.0625)
    localparam signed [31:0] C  = 32'h0001_0000; // Constant c = 1.0
    localparam signed [31:0] A0 = 32'h0001_0000; // a0 = 1.0
    
    // State Memory
    logic signed [31:0] x [N_ASSETS-1:0];
    logic signed [31:0] y [N_ASSETS-1:0];
    
    logic [7:0] iter_cnt;
    logic       running;
    
    // Bifurcation parameter (increases slowly over time)
    // a(t) = a_step * t
    localparam signed [31:0] A_STEP = 32'h0000_0800; // a_step
    logic signed [31:0] a_t;
    
    // Internal Flags for overflow
    logic local_ovf;
    
    // Q16.16 Multiply Macro/Function
    function automatic logic signed [31:0] q_mult(input logic signed [31:0] a, input logic signed [31:0] b);
        logic signed [63:0] temp;
        begin
            temp = $signed(a) * $signed(b);
            // Shift back by 16 bits to maintain Q16.16 alignment
            q_mult = temp[47:16];
        end
    endfunction
    
    always_ff @(posedge clk) begin
        if (rst) begin
            iter_cnt <= 8'd0;
            running  <= 1'b0;
            done     <= 1'b0;
            errant_momentum_ovf <= 1'b0;
            a_t      <= 32'd0;
            for (int i=0; i<N_ASSETS; i++) begin
                x[i] <= 32'd0;
                y[i] <= 32'd0; 
                x_out[i] <= 32'd0;
            end
        end else begin
            if (trigger && !running) begin
                running  <= 1'b1;
                done     <= 1'b0;
                iter_cnt <= 8'd0;
                a_t      <= 32'd0;
                errant_momentum_ovf <= 1'b0;
                for (int i=0; i<N_ASSETS; i++) begin
                    // Small noise injection to break initial symmetry
                    x[i] <= 32'h0000_0001; 
                    y[i] <= 32'h0000_0001; 
                end
            end else if (running) begin
                if (iter_cnt == MAX_ITER) begin
                    running <= 1'b0;
                    done    <= 1'b1;
                    $display("BSB_CORE: DONE at iter_cnt %0d. x[0] = %h", iter_cnt, x[0]);
                    for (int i=0; i<N_ASSETS; i++) begin
                        x_out[i] <= x[i];
                    end
                end else begin
                    local_ovf = 0;
                    
                    // Euler Integration Step Unrolled
                    // Note: Synthesis tools will pipeline this inner loop across DSPs
                    for (int i=0; i<N_ASSETS; i++) begin
                        logic signed [31:0] sum_jx;
                        logic signed [31:0] dy;
                        logic signed [31:0] next_y;
                        logic signed [31:0] next_x;
                        
                        logic signed [63:0] dy_full;
                        logic signed [63:0] next_y_full;
                        
                        // 1. Matrix-Vector Multiplication: sum(J_ij * x_j)
                        sum_jx = 0;
                        for (int j=0; j<N_ASSETS; j++) begin
                            sum_jx = sum_jx + q_mult(j_sigma[i][j], x[j]);
                        end
                        
                        // 2. Momentum Derivative: dy/dt = -(a0 - a(t))*x_i + c*sum_jx + c*h_i
                        dy_full = $signed(q_mult(-(A0 - a_t), x[i])) + 
                                  $signed(q_mult(C, sum_jx)) + 
                                  $signed(q_mult(C, h_mu[i]));
                        dy = dy_full[31:0];
                        
                        // 3. Update Momentum: y_next = y_t + dt * dy
                        next_y_full = $signed(y[i]) + $signed(q_mult(DT, dy));
                        next_y = next_y_full[31:0];
                        
                        // Overflow Detection Danger Zone
                        // Trap any integration step that busts out of the Q16.16 32-bit signed limits
                        if (dy_full > 64'sh0000_0000_7FFF_FFFF || dy_full < -64'sh0000_0000_8000_0000) begin
                            local_ovf = 1;
                        end
                        if (next_y_full > 64'sh0000_0000_7FFF_FFFF || next_y_full < -64'sh0000_0000_8000_0000) begin
                            local_ovf = 1;
                        end
                        
                        // 4. Update Position: x_next = x_t + dt * y_next
                        next_x = x[i] + q_mult(DT, next_y);
                        
                        if (iter_cnt < 2 && i == 0) begin
                            $display("Time=%0t iter=%0d i=%0d x=%h y=%h a_t=%h h_mu=%h sum_jx=%h dy=%h next_y=%h next_x=%h", 
                                     $time, iter_cnt, i, x[i], y[i], a_t, h_mu[i], sum_jx, dy, next_y, next_x);
                        end
                        
                        y[i] <= next_y;
                        x[i] <= next_x;
                    end
                    
                    if (local_ovf) begin
                        errant_momentum_ovf <= 1'b1;
                    end
                    
                    // Increment a(t) and loop counter
                    a_t <= a_t + A_STEP;
                    iter_cnt <= iter_cnt + 1;
                end
            end
        end
    end
endmodule
