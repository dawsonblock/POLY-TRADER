/*
 * APEX WAR MACHINE v4.0 - C++ ROUTER
 * Purpose: Bypasses Python for direct UDP socket dispatch to Jito (Solana) 
 * or Bloxroute (Polygon) mempools after the FPGA completes hardware signing.
 */

#include <iostream>
#include <sys/socket.h>
#include <netinet/in.h>
#include <arpa/inet.h>
#include <fcntl.h>
#include <unistd.h>
#include <cstring>

#define FPGA_DMA_DEVICE "/dev/xdma0_c2h_0" // Card-to-Host DMA
#define JITO_RPC_IP "10.0.1.55" // Local NY4 cross-connect IP
#define JITO_PORT 11222

int main() {
    std::cout << "[CPP ROUTER] Arming Direct Mempool Injections." << std::endl;

    // Open UDP Socket with kernel bypass (e.g., Solarflare Onload if available)
    int sock = socket(AF_INET, SOCK_DGRAM, 0);
    struct sockaddr_in target_addr;
    target_addr.sin_family = AF_INET;
    target_addr.sin_port = htons(JITO_PORT);
    inet_pton(AF_INET, JITO_RPC_IP, &target_addr.sin_addr);

    // Open DMA to read FPGA signatures
    int fpga_fd = open(FPGA_DMA_DEVICE, O_RDONLY);
    if (fpga_fd < 0) {
        std::cerr << "FATAL: Cannot map FPGA PCIe interface." << std::endl;
        return 1;
    }

    uint8_t signed_tx_buffer[512];

    while (true) {
        // Blocking read - waits for FPGA interrupt
        ssize_t bytes_read = pread(fpga_fd, signed_tx_buffer, sizeof(signed_tx_buffer), 0x20000);
        
        if (bytes_read > 0) {
            // Blast signed payload to RPC in < 1 microsecond
            sendto(sock, signed_tx_buffer, bytes_read, 0, 
                  (struct sockaddr*)&target_addr, sizeof(target_addr));
            
            // Log for UI
            std::cout << "TX_SENT | BYTES: " << bytes_read << std::endl;
        }
    }

    close(fpga_fd);
    close(sock);
    return 0;
}
