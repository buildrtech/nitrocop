expect { foo }.not_to output.to_stderr

expect { foo }.not_to output.to_stdout

expect { bar }.to output.to_stderr

expect { bar }.to output.to_stdout
