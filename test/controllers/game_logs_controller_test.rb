require 'test_helper'

class GameLogsControllerTest < ActionDispatch::IntegrationTest

  def setup
    @game_log = create( :game_log )
  end

  test 'should get show for different cases' do
    get game_log_url( @game_log.adventure )
    assert_response :success

    @game_log_fight = create( :game_log_fight )
    get game_log_url( @game_log_fight.adventure )
    assert_response :success

    @game_log_adventure_update = create( :game_log_adventure_update )
    get game_log_url( @game_log_adventure_update.adventure )
    assert_response :success
  end

end
