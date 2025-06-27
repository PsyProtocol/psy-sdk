import styled from "styled-components";
import { loadConfig } from '../../config';

const config = loadConfig();

export const CityRollupLogoCon = styled.div`
  display: flex;
  flex-direction: row;
  align-items: center;
  justify-content: center;
  width: 100%;
  padding-top: 24px;
  margin-top: 24px;
  border-top: 1px solid rgba(200, 200, 200, 0.1);
  
  img {
    height: 140px;
  }
`;

export const SettingsButtonContainer = styled.div`
  position: absolute;
  top: 20px;
  right: 20px;
  z-index: 1000;
`;

export const SettingsButton = styled.button`
  background: none;
  border: none;
  color: ${config.theme.colors.text};
  cursor: pointer;
  padding: 8px;
  border-radius: 8px;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.2s;
  background-color: rgba(0, 0, 0, 0.1);
  backdrop-filter: blur(10px);
  
  &:hover {
    background-color: rgba(115, 231, 255, 0.1);
    transform: scale(1.05);
  }
`;