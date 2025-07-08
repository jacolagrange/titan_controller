import configparser
from pathlib import Path
from typing import List, Dict, Tuple

from process_scripts.sim_information import SimulatorInformation, SimData

# ---------------------------------------------- Config related method ---------------------------------------------------

class SimCfg(SimulatorInformation):
    def __init__(self, dir_path: Path):
        self.file_path = dir_path / "sim.cfg"

    def get(self) -> List[SimData]:
        config = configparser.ConfigParser()
        config.read(self.file_path)
    
        values = []
        for section in config.sections():
            for key, value in config.items(section):
                values.append(SimData("config", section, key, None, value))
    
        return values
