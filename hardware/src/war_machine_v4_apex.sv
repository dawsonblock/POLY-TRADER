/*
 * WAR MACHINE v4.0 (APEX BUILD)
 * - 100GbE Physical Coding Sublayer (PCS) Scraper
 * - 2000-Dimensional bSB Physics Engine
 * - Dynamic Margin HEAB Gate
 */

module war_machine_v4_apex #(
    parameter N_ASSETS = 2000, // Quadrupled from v3.0
    parameter BIT_WIDTH = 8    // Custom FP8 for massive BRAM packing
)(
    input  logic sys_clk_500mhz,  // Overclocked DSP domain
    
    // 100GbE Optical Interface (Direct QSFP28 Pins)
    input  logic [3:0] qsfp_rx_p,
    input  logic [3:0] qsfp_rx_n,
    
    // PCIe Gen5 x16 DMA Bridge
    input  logic [511:0] pcie_dma_data,
    input  logic         pcie_dma_valid,
    
    // Physical Kill Switch
    output logic ssr_optical_kill
);

    // =========================================================================
    // 1. 100GbE PCS TICK SCRAPER (Zero-MAC Latency)
    // =========================================================================
    // We do not wait for the Ethernet packet to form. We look for the exact 
    // binary signature of a Kalshi/Binance price update directly in the bitstream.
    logic [63:0] raw_price_data;
    logic        tick_detected;

    pcs_pattern_matcher matcher_inst (
        .clk(sys_clk_500mhz),
        .rx_lanes_p(qsfp_rx_p),
        .target_signature(64'hBEAF_DEAD_MARKET_TICK),
        .extracted_payload(raw_price_data),
        .valid(tick_detected)
    );

    // =========================================================================
    // 2. 2000-DIMENSIONAL SBM (FP8 Systolic Array)
    // =========================================================================
    // Note: DMA writes FP8 into URAM. For architectural simulation, we assume
    // type casting converts FP8 into Q16.16 fixed-point representation.
    logic signed [31:0] mu_uram [N_ASSETS-1:0];
    logic signed [31:0] sigma_uram [N_ASSETS-1:0][N_ASSETS-1:0];
    logic signed [31:0] portfolio_state_q16 [N_ASSETS-1:0];
    logic sbm_converged;
    logic errant_momentum_ovf;

    bsb_core #(
        .N_ASSETS(N_ASSETS),
        .MAX_ITER(100)
    ) bsb_inst (
        .clk(sys_clk_500mhz),
        .rst(1'b0), // Tied to 0 for stub simulation
        .trigger(tick_detected),
        .h_mu(mu_uram),
        .j_sigma(sigma_uram),
        .x_out(portfolio_state_q16),
        .done(sbm_converged),
        .errant_momentum_ovf(errant_momentum_ovf)
    );

    // =========================================================================
    // 3. DYNAMIC MARGIN HEAB GATE
    // =========================================================================
    logic [31:0] current_delta_exposure;
    logic [31:0] dynamic_max_notional;
    logic        heab_optical_kill;

    heab_gate #(
        .N_ASSETS(N_ASSETS)
    ) heab_inst (
        .clk(sys_clk_500mhz),
        .rst(1'b0),
        .x_final(portfolio_state_q16),
        .bsb_done(sbm_converged),
        .errant_momentum_ovf(errant_momentum_ovf),
        .current_delta_exposure(current_delta_exposure),
        .dynamic_max_notional(dynamic_max_notional),
        .optical_kill(heab_optical_kill)
    );

    always_comb begin
        // Safety Override
        if (raw_price_data == 64'hFFFFFFFFFFFFFFFF) begin // Malformed exchange packet
            ssr_optical_kill = 1'b0; // Drop power to optical transceivers
        end else begin
            // HEAB kill is active high inside module, but ssr_optical_kill is active low
            // Assuming 0 is kill, 1 is live based on prior physical hookup
            ssr_optical_kill = heab_optical_kill ? 1'b0 : 1'b1;
        end
    end

endmodule
