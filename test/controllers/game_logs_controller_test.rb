require 'test_helper'

class GameLogsControllerTest < ActionDispatch::IntegrationTest

  setup do
    # @request.env['devise.mapping'] = Devise.mappings[:user]
    @user = create(:user)
    sign_in @user

    @book = create( :book )
    @adventure = create( :adventure, book: @book, current_page: @book.first_page, user: @user )
    @game_log = create( :game_log, adventure: @adventure, page: @book.first_page )
  end

  test 'should get show for different cases' do
    get game_logs_url
    assert_response :success

    @game_log_fight = create( :game_log_fight, adventure: @adventure, page: @book.first_page )
    get game_logs_url( @game_log_fight.adventure )
    assert_response :success

    @game_log_adventure_update = create( :game_log_adventure_update, adventure: @adventure, page: @book.first_page )
    get game_logs_url( @game_log_adventure_update.adventure )
    assert_response :success
  end

end
