import styled from 'styled-components';

export const AddressHeaderContainer = styled.div`
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: ${({ theme }) => theme.spacing.md} ${({ theme }) => theme.spacing.sm};
`;

export const AddressHeaderItem = styled.div`
  display: flex;
  padding: ${({ theme }) => theme.spacing.md} 0 ${({ theme }) => theme.spacing.lg} 0;
  width: 100%;
  border-bottom: 1px solid ${({ theme }) => theme.colors.border};
  flex-direction: column;
  align-items: center;
  justify-content: center;

  &:first-child {
    padding-top: ${({ theme }) => theme.spacing.xs};
  }
`;

export const AddressHint = styled.div`
  font-size: 12px;
  font-weight: 300;
  color: ${({ theme }) => theme.colors.secondary};
  opacity: 0.7;
`;

export const AddressValue = styled.div`
  font-size: 16px;
  font-weight: 400;
  color: ${({ theme }) => theme.colors.text};
  display: flex;
  flex-direction: row;
  align-items: center;
  justify-content: center;
  gap: ${({ theme }) => theme.spacing.xs};
`;

export const InnerValue = styled.span`
  padding-right: 2px;
`;

export const NoWalletAddressHeader = styled.div`
  display: block;
  text-align: center;
  margin: 0;
  padding: ${({ theme }) => theme.spacing.lg} ${({ theme }) => theme.spacing.sm};
  font-size: 24px;
  font-weight: 300;
  color: ${({ theme }) => theme.colors.secondary};
  opacity: 0.8;
`;