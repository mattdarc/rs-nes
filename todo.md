Notes

Dependency graph:



                           +----- *R* ----> Controller
                           |
                           |
                           +--- *R* ---> **PRG ROM** (Cartridge, accessed by CPU)
                           |
**VNES** ----> **CPU** -- *R/W* --> **Bus** --- *R/W* ---> **CPU RAM**
                           |
                           |
                           +---- *R/W* ----> **CHR RAM** (Cartridge, accessed by CPU/PPU)
                           |
                           |
                           +---- *R/W* ---> **PPU** ---- *R/W* ----> **VRAM**
                           |
                           |
                           |        
                           +--- *R* --> **CHR ROM** (Cartridge, accessed by PPU)


* Cartridge splits up 
