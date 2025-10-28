import styled from 'styled-components';
import { loadConfig } from '../../config';

const config = loadConfig();

export const BalanceContainer = styled.div`
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  padding: 12px 16px;
  margin: 16px 16px 16px 16px;
  background: linear-gradient(135deg, ${config.theme.colors.primary}10, ${config.theme.colors.accent}08);
  border: 1px solid ${config.theme.colors.border};
  border-radius: 8px;
`;

export const BalanceAmount = styled.span`
  color: ${config.theme.colors.text};
  font-size: 20px;
  font-weight: 600;
  letter-spacing: 0.5px;
`;

export const BalanceCurrency = styled.span`
  color: ${config.theme.colors.text};
  font-size: 16px;
  font-weight: 500;
  opacity: 0.8;
`;