import styled from 'styled-components';
import { UnstyledButton } from '@mantine/core';

export const StyledWalletActionButton = styled(UnstyledButton)`
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 6px;
  font-size: 12px;
  font-weight: 300;
  color: var(--mantine-color-gray-1);
  border: 1px solid transparent;
  background: transparent;
  text-align: center;
  width: 100%;

  &:hover {
    border-color: light-dark(var(--mantine-color-dark-8), var(--mantine-color-gray-8));
  }

  &:active {
    background: light-dark(var(--mantine-color-dark-8), var(--mantine-color-gray-8));
  }
`;

export const Icon = styled.div`
  height: 24px;
`;

export const Label = styled.div`
  margin-top: 4px;
`;