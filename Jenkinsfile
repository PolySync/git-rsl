#!groovy
node('xenial') {
	stage('Checkout') {
	  clean_checkout()
	}
	stage('Build') {
	  sh 'cargo build'
	}
	stage('Test') {
	  sh 'cargo test --no-fail-fast -- --nocapture'
	}
}
