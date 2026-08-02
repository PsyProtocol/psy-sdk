import { PsyNetworkConfig, PsyConfig } from "@psy-protocol/psy-sdk";

import * as rootConfig from "../../../../../../config.json";
const psyConfig = rootConfig as PsyConfig;

const networkConfig = psyConfig.networks[psyConfig.defaultNetwork];

export { PsyNetworkConfig, networkConfig };
