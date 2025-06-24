import styled from 'styled-components';
import { loadConfig } from '../../config';

const config = loadConfig();

export const ExtensionWalletContainer = styled.div`
  background-color: ${config.theme.colors.background};
  width: 100%;
  height: 100%;
  display: flex;
  flex-direction: column;
  position: relative;
`;