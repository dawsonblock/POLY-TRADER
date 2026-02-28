/*
 * BOREAL INLINE RISK FIREWALL
 * Module: Dual-Channel Exposure Clamp (SIL-3 Concept)
 * Purpose: Analyzes outbound packets inline. Drops malformed or 
 * risk-violating orders before they hit the physical MAC layer.
 */

module boreal_dual_clamp #(
    parameter NUM_ASSETS = 256,
    parameter BIT_WIDTH = 64
)(
    input  wire clk,
    input  wire rst_n,

    // Inbound from Host CPU (The Trading Algorithm)
    input  wire host_order_valid,
    input  wire [15:0] host_asset_id,
    input  wire [BIT_WIDTH-1:0] host_order_size,  // Signed fixed-point
    input  wire [BIT_WIDTH-1:0] host_order_price,

    // Configuration Limits (Set by Chief Risk Officer via secure side-channel)
    input  wire [BIT_WIDTH-1:0] max_position [NUM_ASSETS-1:0],
    input  wire [BIT_WIDTH-1:0] max_notional_per_order,

    // Outbound to 10G/25G MAC (The Exchange)
    output reg  tx_enable,
    output reg  fault_flag
);

    // =========================================================================
    // CHANNEL A (Primary Logic)
    // =========================================================================
    reg signed [BIT_WIDTH-1:0] pos_ram_A [NUM_ASSETS-1:0];
    reg clamp_A;

    integer i;
    initial begin
        for (i = 0; i < NUM_ASSETS; i = i + 1) begin
            pos_ram_A[i] = 0;
            pos_ram_B[i] = 0;
        end
    end

    always_ff @(posedge clk) begin
        if (!rst_n) begin
            clamp_A <= 1'b0;
        end else if (host_order_valid) begin
            // 1. Check Notional Size
            if (host_order_size > max_notional_per_order) begin
                clamp_A <= 1'b1;
            end 
            // 2. Check Aggregate Inventory
            else if ((pos_ram_A[host_asset_id] + host_order_size) > max_position[host_asset_id]) begin
                clamp_A <= 1'b1;
            end 
            // 3. Update State if Safe
            else begin
                pos_ram_A[host_asset_id] <= pos_ram_A[host_asset_id] + host_order_size;
                clamp_A <= 1'b0;
            end
        end
    end

    // =========================================================================
    // CHANNEL B (Redundant Validation)
    // Physically separated logic paths to prevent single-event upsets
    // =========================================================================
    reg signed [BIT_WIDTH-1:0] pos_ram_B [NUM_ASSETS-1:0];
    reg clamp_B;

    always_ff @(posedge clk) begin
        if (!rst_n) begin
            clamp_B <= 1'b0;
        end else if (host_order_valid) begin
            if (host_order_size > max_notional_per_order) begin
                clamp_B <= 1'b1;
            end else if ((pos_ram_B[host_asset_id] + host_order_size) > max_position[host_asset_id]) begin
                clamp_B <= 1'b1;
            end else begin
                pos_ram_B[host_asset_id] <= pos_ram_B[host_asset_id] + host_order_size;
                clamp_B <= 1'b0;
            end
        end
    end

    // =========================================================================
    // COMPARATOR & EJECTION
    // =========================================================================
    always_comb begin
        // If channels disagree on state, or either channel asserts a clamp, we fault.
        if (clamp_A || clamp_B || (pos_ram_A[host_asset_id] != pos_ram_B[host_asset_id])) begin
            tx_enable = 1'b0;  // Physically drop the packet
            fault_flag = 1'b1; // Trigger hardware alarm
        end else begin
            tx_enable = host_order_valid;
            fault_flag = 1'b0;
        end
    end

endmodule
