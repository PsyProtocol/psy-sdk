import styled from 'styled-components';

export const FaucetFromWallet = styled.div`
  position: relative;
  top: 0;
  left: 0;
  width: 100%;
`;

export const ModalTitle = styled.div`
  font-size: 20px;
  font-weight: 300;
  margin-bottom: 20px;
`;

export const BlokiesCon = styled.div`
  display: flex;
  flex-direction: row;
  align-items: center;
  justify-content: center;
  padding-top: 8px;
  padding-bottom: 12px;
`;

export const BlokiesIcon = styled.div`
  border-radius: 8px;
`;

export const FaucetFromWalletForm = styled.div`
  position: relative;
  top: 0;
  left: 0;
  width: 100%;
`;

export const FormControls = styled.div`
  position: relative;
  top: 0;
  left: 0;
  width: 100%;
  display: flex;
  flex-direction: row;
  align-items: center;
  justify-content: flex-end;
  padding-top: 24px;
  width: 100%;
`;

export const InputCon = styled.div`
  padding-top: 12px;
  
  &:first-child {
    padding-top: 0px;
  }
  
  input {
    font-family: var(--wallet-widget-font-mono) !important;
  }
`;

export const HelpText = styled.div`
  font-size: 12px;
  color: var(--mantine-color-gray-6);
  margin-top: 4px;
`;