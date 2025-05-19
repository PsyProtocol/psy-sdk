import subprocess
import json
import os
import logging
import re

logging.basicConfig(level=logging.INFO, format='%(asctime)s - %(name)s - %(levelname)s - %(message)s')
logger = logging.getLogger(__name__)

class QEDClient:
    def __init__(self, profile="release", log_level="debug"):
        self.profile = profile
        self.log_level = log_level
        self.project_dir = os.path.dirname(os.path.abspath(__file__))
        self.target_dir = os.path.join(self.project_dir, "target")
        self.coordinator_rpc_url = "http://127.0.0.1:8545"
        self.realm_rpc_url = "http://127.0.0.1:8546"

        # Predefined private keys from Makefile
        self.predefined_keys = {
            0: "17c975c2668ebe0ca7c87f67c6414ebb7fd664f46370a0af2a3b204c8824ac5a",  # USER0_PRIVATE_KEY
            1: "f07f91a0bdc0df4ec763285ba0eb578cb6e7a0811c3150494ab54e56f761fc1d",  # USER1_PRIVATE_KEY
        }

    def _get_private_key(self, private_key_or_id):
        """Get private key from ID or return the key if it's already a key"""
        if isinstance(private_key_or_id, int):
            if private_key_or_id not in self.predefined_keys:
                raise ValueError(f"Private key ID {private_key_or_id} not found in predefined keys")
            return self.predefined_keys[private_key_or_id]
        return private_key_or_id

    def _clean_ansi_codes(self, text):
        """Remove ANSI color codes from text"""
        ansi_escape = re.compile(r'\x1B(?:[@-Z\\-_]|\[[0-?]*[ -/]*[@-~])')
        return ansi_escape.sub('', text)

    def _extract_public_key(self, output):
        """Extract l2 public key from command output"""
        clean_output = self._clean_ansi_codes(output)
        # Try to match the l2 public key line
        match = re.search(r'l2 public_key = (\w+)', clean_output)
        if match:
            return match.group(1)

        # If not found, try to get the last line which might contain the key
        lines = clean_output.strip().split('\n')
        for line in reversed(lines):
            if line.strip():
                # If it's a hex string that might be a public key
                if re.match(r'^[0-9a-f]+$', line.strip()):
                    return line.strip()

        # If no suitable match is found, return the cleaned output
        logger.warning("Could not extract public key cleanly, returning full output")
        return clean_output

    def _run_command(self, command):
        """Execute command and return output"""
        logger.debug(f"Running command: {command}")
        try:
            result = subprocess.run(command, shell=True, check=True, capture_output=True, text=True)
            return result.stdout
        except subprocess.CalledProcessError as e:
            logger.error(f"Command failed: {e}")
            logger.error(f"Error output: {e.stderr}")
            raise

    def _run_curl(self, url, method, params):
        """Execute curl command to send RPC request"""
        params_json = json.dumps(params)
        command = f'curl -s -X POST "{url}" -H "Content-Type: application/json" -d \'{{"jsonrpc": "2.0", "method": "{method}", "params": {params_json}, "id": 1}}\''
        result = self._run_command(command)
        return json.loads(result)

    def get_public_key(self, private_key_or_id):
        """Get public key"""
        private_key = self._get_private_key(private_key_or_id)
        cmd = f"RUST_LOG={self.log_level} {self.target_dir}/{self.profile}/qed_user_cli get-public-key --private-key={private_key}"
        output = self._run_command(cmd)

        # Extract and return just the public key
        return self._extract_public_key(output)

    def random_wallet(self):
        """Generate random wallet"""
        cmd = f"RUST_LOG={self.log_level} {self.target_dir}/{self.profile}/qed_user_cli random-wallet"
        output = self._run_command(cmd)
        return output.strip()

    def mint(self, private_key_or_id, contract_id=0, amount=1000, nonce=1):
        """Mint tokens"""
        private_key = self._get_private_key(private_key_or_id)
        cmd = f"RUST_LOG={self.log_level} {self.target_dir}/{self.profile}/qed_user_cli submit-end-caproof -p {private_key} --contract-id {contract_id} --method-name simple_mint --inputs {amount} --nonce {nonce}"
        output = self._run_command(cmd)
        return output.strip()

    def transfer(self, private_key_or_id, receiver_id=536870912, amount=500, contract_id=0, nonce=2):
        """Transfer tokens"""
        private_key = self._get_private_key(private_key_or_id)
        cmd = f"RUST_LOG={self.log_level} {self.target_dir}/{self.profile}/qed_user_cli submit-end-caproof -p {private_key} --contract-id {contract_id} --method-name simple_transfer --inputs {receiver_id} --inputs {amount} --nonce {nonce}"
        output = self._run_command(cmd)
        return output.strip()

    def claim(self, private_key_or_id, contract_id=0, from_id=0, nonce=1):
        """Claim tokens"""
        private_key = self._get_private_key(private_key_or_id)
        cmd = f"RUST_LOG={self.log_level} {self.target_dir}/{self.profile}/qed_user_cli submit-end-caproof -p {private_key} --contract-id {contract_id} --method-name simple_claim --inputs {from_id} --nonce {nonce}"
        output = self._run_command(cmd)
        return output.strip()

    def return_back(self, private_key_or_id, receiver_id=0, amount=500, contract_id=0, nonce=2):
        """Return tokens"""
        private_key = self._get_private_key(private_key_or_id)
        cmd = f"RUST_LOG={self.log_level} {self.target_dir}/{self.profile}/qed_user_cli submit-end-caproof -p {private_key} --contract-id {contract_id} --method-name simple_transfer --inputs {receiver_id} --inputs {amount} --nonce {nonce}"
        output = self._run_command(cmd)
        return output.strip()

    def balance_of(self, checkpoint_id=1, user_id=0, contract_id=0, contract_state_height=24, slot_id=0):
        """Query balance"""
        return self._run_curl(
            self.realm_rpc_url,
            "qed_get_user_contract_state_tree_merkle_proof",
            [checkpoint_id, user_id, contract_id, contract_state_height, slot_id]
        )

    def build_block(self):
        """Build block"""
        return self._run_curl(self.coordinator_rpc_url, "qed_build_block", [])

    def latest_checkpoint(self):
        """Get latest checkpoint"""
        return self._run_curl(self.coordinator_rpc_url, "qed_latest_checkpoint", [])

    def register_user(self, public_key_param, fingerprint):
        """Register user"""
        params = {"fingerprint": fingerprint, "public_key_param": public_key_param}
        return self._run_curl(self.coordinator_rpc_url, "qed_register_user", params)

    def deploy_contract(self, private_key_or_id, contract_path=None):
        """Deploy contract"""
        private_key = self._get_private_key(private_key_or_id)
        if contract_path is None:
            contract_path = f"{self.project_dir}/examples/target/examples.json"

        cmd = f"RUST_LOG={self.log_level} {self.target_dir}/{self.profile}/qed_user_cli deploy-contract --private-key={private_key} --contract-path {contract_path}"
        output = self._run_command(cmd)
        return output.strip()

    def add_private_key(self, key_id, private_key):
        """Add a new private key to the predefined keys dictionary"""
        self.predefined_keys[key_id] = private_key
        return True

    def get_predefined_keys(self):
        """Get all predefined private keys"""
        return self.predefined_keys.copy()

    def get_contract_leaf_data(self, contract_id=0):
        """Get contract leaf data"""
        return self._run_curl(self.coordinator_rpc_url, "qed_get_contract_leaf_data", [contract_id])

    def get_checkpoint_leaf_data(self, checkpoint_id=1):
        """Get checkpoint leaf data"""
        return self._run_curl(self.coordinator_rpc_url, "qed_get_checkpoint_leaf_data", [checkpoint_id])

    def get_checkpoint_global_state_roots(self, checkpoint_id=1):
        """Get checkpoint global state roots"""
        return self._run_curl(self.coordinator_rpc_url, "qed_get_checkpoint_global_state_roots", [checkpoint_id])

    def get_checkpoint_tree_root(self, checkpoint_id=1):
        """Get checkpoint tree root"""
        return self._run_curl(self.coordinator_rpc_url, "qed_get_checkpoint_tree_root", [checkpoint_id])

    def get_user_tree_merkle_proof(self, checkpoint_id=1, user_id=0):
        """Get user tree Merkle proof"""
        return self._run_curl(self.coordinator_rpc_url, "qed_get_user_tree_merkle_proof", [checkpoint_id, user_id])


# Example usage
if __name__ == "__main__":
    client = QEDClient()

    public_key_0 = client.get_public_key(0)
    print(f"Public key for user 0: {public_key_0}")
