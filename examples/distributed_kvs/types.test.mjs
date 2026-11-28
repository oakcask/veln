import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { copyFile, mkdtemp, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { test } from 'node:test';

const root = fileURLToPath(new URL('../../', import.meta.url));
const source = fileURLToPath(new URL('./veln/', import.meta.url));
const request = 'domain::ClientRequest(domain::RequestId(7, 1), domain::Put("61", "62"))';
const vote = 'protocol::RequestVote(1, 1, protocol::LogPosition(0, 0))';
const append = 'protocol::AppendEntries(1, 1, protocol::LogPosition(0, 0), 0, None)';

// An ordinary importing module must not receive exact-companion private access.
// Compile the real example modules, so these checks follow the exported API.
test('Raft role and completion boundaries reject invalid source programs', { timeout: 60000 }, async t => {
  const directory = await mkdtemp(join(tmpdir(), 'veln-kvs-types-'));
  t.after(() => rm(directory, { recursive: true, force: true }));
  await copyFile(join(source, 'veln.toml'), join(directory, 'veln.toml'));
  for (const name of ['command.veln', 'protocol.veln', 'raft.veln']) {
    await copyFile(join(source, name), join(directory, name));
  }
  const binary = resolve(process.env.VELN_BIN ?? join(root, 'target/debug/veln'));
  async function check(body) {
    await writeFile(join(directory, 'main.veln'), `use command as domain\nuse protocol\nuse raft\n\n${body}\n`);
    const result = spawnSync(binary, ['check', '.'], { cwd: directory, encoding: 'utf8', timeout: 15000 });
    assert.ifError(result.error);
    assert.equal(result.signal, null);
    return { status: result.status, output: result.stdout + result.stderr };
  }
  const valid = await check(`
pub fn propose(s: raft::State::Leader) -> raft::State::Leader
  raft::propose(s, ${request})
end
pub fn dispatch(s: raft::State) -> Option<protocol::PeerRequest>
  match s
    raft::Follower(_) => None
    raft::Candidate(_) => Some(raft::vote_request(s))
    raft::Leader(_) => Some(raft::replication(s, 2))
  end
end
pub fn campaign(s: raft::State::Follower | raft::State::Candidate) -> raft::State::Candidate
  raft::campaign(s, 10)
end
pub fn vote_completed(sent: protocol::PeerRequest::RequestVote, reply: protocol::PeerReply::VoteReply) -> protocol::PeerCompletion
  protocol::VoteCompleted(sent, reply)
end
pub fn append_completed(sent: protocol::PeerRequest::AppendEntries, reply: protocol::PeerReply::AppendReply) -> protocol::PeerCompletion
  protocol::AppendCompleted(sent, reply)
end`);
  assert.equal(valid.status, 0, valid.output);
  const invalid = [
    ['follower proposal', `pub fn misuse() -> raft::State\n  raft::propose(raft::initial(1, 0), ${request})\nend`, 'type.variant_mismatch'],
    ['follower replication', 'pub fn misuse() -> protocol::PeerRequest\n  raft::replication(raft::initial(1, 0), 2)\nend', 'type.variant_mismatch'],
    ['leader election', 'pub fn misuse(s: raft::State::Leader) -> raft::State\n  raft::campaign(s, 10)\nend', 'type.variant_mismatch'],
    ['follower vote request', 'pub fn misuse() -> protocol::PeerRequest\n  raft::vote_request(raft::initial(1, 0))\nend', 'type.variant_mismatch'],
    ['role payload substitution', 'pub fn misuse() -> raft::State\n  match raft::initial(1, 0)\n    raft::Follower(payload) => raft::Leader(payload)\n  end\nend', 'type.mismatch'],
    ['sparse log construction', 'pub fn misuse() -> raft::ReplicatedLog\n  raft::ReplicatedLog(5, {})\nend', 'name.unresolved'],
    ['persistent construction', 'pub fn misuse(log: raft::ReplicatedLog) -> raft::PersistentState\n  raft::PersistentState(0, None, log)\nend', 'name.unresolved'],
    ['opaque leader construction', 'pub fn misuse() -> raft::LeaderState\n  raft::LeaderState(raft::initial(1, 0))\nend', 'name.unresolved'],
    ['unchecked commit update', 'pub fn misuse(s: raft::State) -> raft::State\n  raft::with_commit(s, 10)\nend', 'name.unresolved'],
    ['unelected promotion', 'pub fn misuse() -> raft::State\n  raft::lead(raft::campaign(raft::initial(1, 0), 10))\nend', 'name.unresolved'],
    ['vote with append reply', `pub fn misuse() -> protocol::PeerCompletion\n  protocol::VoteCompleted(${vote}, protocol::AppendReply(1, 2, true, 100))\nend`, 'type.mismatch'],
    ['append with vote reply', `pub fn misuse() -> protocol::PeerCompletion\n  protocol::AppendCompleted(${append}, protocol::VoteReply(1, 2, true))\nend`, 'type.mismatch'],
  ];
  for (const [label, body, diagnostic] of invalid) {
    await t.test(label, async () => {
      const result = await check(body);
      assert.notEqual(result.status, 0, `${label} unexpectedly type-checked`);
      assert.ok(result.output.includes(`error[${diagnostic}]`), result.output);
    });
  }
});
