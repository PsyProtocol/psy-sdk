'use strict';

exports.SignType = void 0;
(function (SignType) {
    SignType["ZKSign"] = "zk";
    SignType["SECP256K1Sign"] = "secp256k1";
    SignType["SoftwareDefinedDPNSign"] = "software-defined-dpn";
    SignType["SoftwareDefinedPlonky2Sign"] = "software-defined-plonky2";
    SignType["SDKeySign"] = "sd-key";
})(exports.SignType || (exports.SignType = {}));
