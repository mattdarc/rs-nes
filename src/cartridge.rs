use crate::mapper::*;
use std::io::Read;

#[derive(Debug, Default, Clone)]
pub struct Cartridge {
    name: String,

    // This may not need to be a box - we can instantiate a new type for each mapper fine
    mapper: Box<dyn Mapper>,
}

impl Cartridge {
    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn load(filename: &str) -> Result<Cartridge, std::io::Error> {
        let mut fh = std::fs::File::open(filename)?;
        let mut header: [u8; 16] = [0; 16];
        fh.read_exact(&mut header)?;
        let header = Header::from(&header);
        println!("Header: {:?}", &header);

        let mut data = vec![0; header.get_prg_rom_size()];
        fh.read_exact(&mut data)?;

        let mapper = create_mapper(&header, &data);
        Ok(Cartridge {
            name: filename.to_owned(),
            mapper,
        })
    }

    pub fn mapper(&self) -> &Box<dyn Mapper> {
        &self.mapper
    }

    pub fn mapper_mut(&mut self) -> &mut Box<dyn Mapper> {
        &mut self.mapper
    }
}

#[cfg(test)]
pub mod test {
    use super::*;
    use crate::mapper::test;

    pub fn program(data: &[u8]) -> Cartridge {
        Cartridge {
	    name: String::default(),
	    mapper: test::mapper_with(data),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::ErrorKind;

    #[test]
    fn load_none() {
        let rom = Cartridge::load("NoFile.nes");
        assert!(rom.is_err());
        assert!(match rom.err() {
            Some(e) => e.kind() == ErrorKind::NotFound,
            None => false,
        });
    }

    #[test]
    fn load_some() {
        let exp_name = "roms/Tetris.nes";
        let cart = match Cartridge::load(exp_name) {
            Ok(cart) => cart,
            Err(e) => unreachable!("Error {:?}", e),
        };
        assert!(cart.mapper.prg_size() > 0);
        assert!(cart.mapper.chr_size() > 0);
        assert_eq!(cart.get_name(), exp_name);
    }
}
