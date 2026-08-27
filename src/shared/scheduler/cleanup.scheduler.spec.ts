import { getCleanupTargets } from './cleanup.scheduler';

it('cleanup target selection', () => {
  const now = new Date('2024-01-01');
  const active = { id: 1, expiresAt: new Date('2025-01-01') };
  const expired = { id: 2, expiresAt: new Date('2023-01-01') };
  const targets = getCleanupTargets([active, expired], now);
  expect(targets).toEqual([expired]);
});
