import semanticRelease from 'semantic-release';
import { WritableStreamBuffer } from "stream-buffers";

const stdoutBuffer = new WritableStreamBuffer();
const stderrBuffer = new WritableStreamBuffer();

try {
  const result = await semanticRelease(
    {
      dryRun: true,
      ci: false,
    },
    {
      stdout: stdoutBuffer,
      stderr: stderrBuffer,
    }
  );

  const version = result?.nextRelease?.version;

  if (version) {
    process.stdout.write(version);
  }

} catch (err) {
  process.stderr.write(`semantic-release error: ${err.message}\n`);
  process.exit(1);
}
