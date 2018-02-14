#!groovy
node('xenial') {
	stage('Checkout') {
	  clean_checkout()
	}
	stage('Build') {
		withEnv(["PATH+CARGO=$HOME/.cargo/bin"]) {
	  	sh 'cargo build'
		}
	}
	stage('Test') {
		withEnv(["PATH+CARGO=$HOME/.cargo/bin"]) {
			sh 'cargo test --no-fail-fast -- --nocapture'
		}
	}
}
