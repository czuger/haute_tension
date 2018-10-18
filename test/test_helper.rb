require 'pp'

require 'simplecov'
SimpleCov.start 'rails' do
  add_filter '/mailers/'
  add_filter '/jobs/'
  add_filter '/application_cable/'
  add_filter '/pages_download.rb'
  add_filter '/pages_update.rb'
end

ENV['RAILS_ENV'] ||= 'test'
require File.expand_path('../../config/environment', __FILE__)
require 'rails/test_help'
require 'mocha/test_unit'

class ActiveSupport::TestCase
  # Setup all fixtures in test/fixtures/*.yml for all tests in alphabetical order.
  # fixtures :all

  # Add more helper methods to be used by all tests here...
  include FactoryBot::Syntax::Methods

  include Devise::Test::IntegrationHelpers

end
