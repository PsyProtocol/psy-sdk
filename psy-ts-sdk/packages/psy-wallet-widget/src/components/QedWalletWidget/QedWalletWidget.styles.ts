import styled from 'styled-components';

export const WalletWidgetContainer = styled.div`
  position: relative;
  display: block;
  width: 100%;
  height: 100%;
  padding: ${({ theme }) => theme.spacing.sm};
  margin: 0;
  background-color: ${({ theme }) => theme.colors.background};
  overflow: hidden;
  font-family: ${({ theme }) => theme.fonts.sans} !important;
`;

export const WalletWidgetHeader = styled.div`
  position: fixed;
  top: 0;
  left: 0;
  width: 100%;
  height: 54px;
  z-index: 9;
  background-color: ${({ theme }) => theme.colors.background};
`;

export const WalletWidgetBody = styled.div`
  width: 100%;
  height: 100%;
  overflow-y: scroll;
  padding-top: 54px;
  
  /* Custom scrollbar for wallet widget */
  &::-webkit-scrollbar {
    width: 6px;
  }
  
  &::-webkit-scrollbar-track {
    background: ${({ theme }) => theme.colors.background};
  }
  
  &::-webkit-scrollbar-thumb {
    background: ${({ theme }) => theme.colors.border};
    border-radius: ${({ theme }) => theme.borderRadius.sm};
  }
`;

export const TransactionContainer = styled.div`
  display: flex;
  width: 100%;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding-top: ${({ theme }) => theme.spacing.sm};
`;